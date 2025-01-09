mod models;
mod services;

#[macro_use]
extern crate prettytable;

use crate::models::*;
use crate::services::*;
use clap::{Parser, Subcommand};
use prettytable::{Cell, Row, Table};
use std::{fs, process};

const TABLE_WRAP_WIDTH: usize = 60;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to local Bicep file (for direct analysis)
    #[arg(long)]
    bicep_file: Option<String>,

    /// Path to best practices MD file (for direct analysis)
    #[arg(long)]
    best_practices_file: Option<String>,

    /// Debug mode
    #[arg(long)]
    debug: Option<bool>,

    /// Minimum severity level (1-5)
    #[arg(long)]
    minimum_severity: Option<u8>,

    /// Simple mode - single prompt without categories
    #[arg(long)]
    simple: Option<bool>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Azure DevOps PR-based analysis
    Azure(AzureDevOpsArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    let cli = Cli::parse();

    // Determine if we're doing local analysis or using a subcommand
    match (cli.bicep_file, cli.command) {
        // Local file analysis
        (Some(bicep_file), None) => {
            let args = Args {
                bicep_file,
                best_practices_file: cli.best_practices_file
                    .ok_or("--best-practices-file is required")?,
                category: None,
                debug: cli.debug.unwrap_or(false),
                minimum_severity: cli.minimum_severity.unwrap_or(DEFAULT_MIN_SEVERITY),
                simple: cli.simple.unwrap_or(false),
            };

            let config = init_local_config(&args)?;
            debug_local_args(&args, config.debug);

            let reviews = analyze_local_bicep(&config).await?;
            let report = finalize_report(reviews, config.minimum_severity).await?;

            println!("\n{}", report);
            if report.contains("5 (Critical)") {
                process::exit(1);
            }
        }

        // Azure DevOps analysis
        (None, Some(Commands::Azure(args))) => {
            if args.debug {
                std::env::set_var("BICEP_DEBUG", "true");
            }
            check_env_vars()?;
            debug_ado_args(&args);
            run_pr_review(args).await?;
        }

        // Invalid combinations
        (Some(_), Some(_)) => {
            return Err("Cannot specify both --bicep-file and a subcommand".into());
        }
        (None, None) => {
            return Err("Must specify either --bicep-file or a subcommand".into());
        }
    }

    Ok(())
}

/// -------------------------------------------------------
/// LOCAL STRUCT & FUNCS

struct LocalConfig {
    bicep_content: String,
    best_practices: String,
    categories: Vec<String>,
    minimum_severity: u8,
    debug: bool,
    simple: bool,
}

/// Build local config from command line Args
fn init_local_config(args: &Args) -> Result<LocalConfig, Box<dyn std::error::Error>> {
    check_local_args(args)?;
    check_env_vars()?;
    set_debug(args.debug);

    let bicep_content = fs::read_to_string(&args.bicep_file)?;
    let best_practices = fs::read_to_string(&args.best_practices_file)?;

    let categories = match &args.category {
        Some(cat) => vec![cat.clone()],
        None => DEFAULT_CATEGORIES.iter().map(|&s| s.to_string()).collect(),
    };

    Ok(LocalConfig {
        bicep_content,
        best_practices,
        categories,
        minimum_severity: args.minimum_severity,
        debug: args.debug,
        simple: args.simple,
    })
}

/// Check arguments
fn check_local_args(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if !(1..=5).contains(&args.minimum_severity) {
        return Err("Minimum severity must be 1..=5".into());
    }
    Ok(())
}

/// Basic environment var check
fn check_env_vars() -> Result<(), Box<dyn std::error::Error>> {
    let needed = [
        "AZURE_OPENAI_ENDPOINT",
        "AZURE_OPENAI_API_KEY",
        "AZURE_OPENAI_DEPLOYMENT",
        "AZURE_SEARCH_ENDPOINT",
        "AZURE_SEARCH_ADMIN_KEY",
        "AZURE_SEARCH_INDEX",
    ];
    let missing: Vec<_> = needed
        .iter()
        .filter(|var| std::env::var(var).is_err())
        .collect();

    if !missing.is_empty() {
        return Err(format!("Missing required env vars: {:?}", missing).into());
    }
    Ok(())
}

fn set_debug(debug: bool) {
    if debug {
        std::env::set_var("BICEP_DEBUG", "true");
    }
}

fn debug_local_args(args: &Args, debug: bool) {
    if debug {
        println!("(DEBUG) Local Args => bicep_file={}, best_practices_file={}, category={:?}, minSeverity={}, debug={}",
            args.bicep_file, args.best_practices_file, args.category, args.minimum_severity, args.debug);
    }
}

/// Analyze local Bicep code
async fn analyze_local_bicep(cfg: &LocalConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if cfg.simple {
        // Simple mode: single prompt
        if cfg.debug {
            println!("(DEBUG) Running simple analysis without categories");
        }
        let result = services::validate_simple(&cfg.bicep_content, &cfg.best_practices).await?;
        Ok(vec![result])
    } else {
        // Original category-based analysis
        let mut reviews = Vec::new();
        for cat in &cfg.categories {
            if cfg.debug {
                println!("(DEBUG) Analyzing category: {}", cat);
            }
            let rev = services::analyze_category(
                &cfg.bicep_content,
                &cfg.best_practices,
                cat,
                cfg.debug,
            )
            .await?;
            reviews.push(rev);
        }
        Ok(reviews)
    }
}

/// Convert reviews into final JSON -> markdown
async fn finalize_report(
    review_texts: Vec<String>,
    min_severity: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    let request = build_final_report_request(&review_texts);
    let response = call_azure_openai(&request).await?;
    let report: FinalReport = serde_json::from_str(&response.choices[0].message.content)?;

    let findings = filter_by_severity(&report, min_severity);
    build_markdown(&findings, min_severity)
}

/// Create the final LLM request
fn build_final_report_request(
    review_texts: &[String],
) -> crate::services::ChatCompletionRequest {
    crate::services::ChatCompletionRequest {
        model: std::env::var("AZURE_OPENAI_DEPLOYMENT").unwrap_or_default(),
        messages: vec![crate::services::ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Convert these review findings into structured JSON.\n\
                 Remove trivial suggestions.\n\
                 Return exactly:\n\
                 {{\n  \"findings\": [\n    {{\n\
                 \"category\": \"...\",\n\"finding\": \"...\",\n\"severity\": 1-5,\n\"impact\": \"...\"\n    }}\n  ]\n}}\n\n\
                 reviews:\n{}",
                review_texts.join("\n\n")
            ),
        }],
        temperature: 0.3,
        response_format: crate::services::ResponseFormat {
            format_type: "json_object".to_string(),
        },
    }
}

/// Filter findings >= minSeverity, sort desc
fn filter_by_severity<'a>(
    report: &'a FinalReport,
    min_severity: u8,
) -> Vec<&'a ValidationResult> {
    let mut out: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity >= min_severity)
        .collect();
    out.sort_by(|a, b| b.severity.cmp(&a.severity));
    out
}

/// Build final markdown table
fn build_markdown(
    findings: &[&ValidationResult],
    min_severity: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut out = String::new();
    out.push_str("# Bicep Code Review Results\n\n");

    if findings.is_empty() {
        out.push_str(&format!(
            "No issues found with severity {} or higher.\n",
            min_severity
        ));
        return Ok(out);
    }

    out.push_str(&format!(
        "Found {} issues with severity {} or higher.\n\n",
        findings.len(),
        min_severity
    ));

    let mut table = Table::new();
    table.add_row(row![b => "Category", "Finding", "Severity", "Impact"]);

    for f in findings {
        let label = match f.severity {
            5 => "5 (Critical) ⚠️",
            4 => "4 (Serious)",
            3 => "3 (Important)",
            2 => "2 (Minor)",
            _ => "1 (Suggestion)",
        };
        let wrapped_find = textwrap::fill(&f.finding, TABLE_WRAP_WIDTH);
        let wrapped_imp = textwrap::fill(&f.impact, TABLE_WRAP_WIDTH);

        table.add_row(Row::new(vec![
            Cell::new(&f.category),
            Cell::new(&wrapped_find),
            Cell::new(label),
            Cell::new(&wrapped_imp),
        ]));
    }

    let table_str = table
        .to_string()
        .lines()
        .map(|line| format!("    {}", line))
        .collect::<Vec<_>>()
        .join("\n");

    out.push_str("```\n");
    out.push_str(&table_str);
    out.push_str("\n```\n");

    if findings.iter().any(|f| f.severity == 5) {
        out.push_str("\n⚠️ **CRITICAL ISSUES FOUND**\n");
    }

    Ok(out)
}

/// -------------------------------------------------------
/// ADO PR-based flow

fn debug_ado_args(args: &AzureDevOpsArgs) {
    if args.debug {
        println!("(DEBUG) Azure DevOps Args => org={}, project={}, repo={}, prId={}, pat=***, bestPractices={}, minSeverity={}, debug={}",
            args.organization, args.project, args.repository, args.pull_request_id, args.best_practices_file, args.minimum_severity, args.debug
        );
    }
}

/// The main function for PR-based analysis
async fn run_pr_review(args: AzureDevOpsArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1) Resolve repo GUID
    let repo_id = get_repository_id(
        &args.organization,
        &args.project,
        &args.repository,
        &args.pat,
    )
    .await?;
    if args.debug {
        println!("(DEBUG) Found repo GUID: {repo_id}");
    }

    // 2) Find changed .bicep files
    let files = list_modified_bicep_files(
        &args.organization,
        &args.project,
        &repo_id,
        args.pull_request_id,
        &args.pat,
        args.debug,
    )
    .await?;

    if files.is_empty() && args.debug {
        println!("(DEBUG) No changed Bicep files in PR #{}, nothing to do", args.pull_request_id);
    }

    // 3) Load best practices
    let best_md = fs::read_to_string(&args.best_practices_file)?;

    // 4) For each changed Bicep file, get content + analyze
    for f in files {
        if args.debug {
            println!("(DEBUG) Reviewing file: {}", f.path);
        }

        let content = get_file_content(
            &args.organization,
            &args.project,
            &repo_id,
            args.pull_request_id,
            &f.path,
            &f.object_id,
            &args.pat,
        )
        .await?;

        if args.debug {
            println!("(DEBUG) Retrieved {} bytes of content for {}", content.len(), f.path);
        }

        let response_content = if args.simple {
            // Simple mode: direct analysis
            if args.debug {
                println!("(DEBUG) Using simple mode analysis");
            }
            services::validate_simple(&content, &best_md).await?
        } else {
            // Category-based analysis
            let mut cat_reviews = Vec::new();
            for cat in DEFAULT_CATEGORIES.iter() {
                println!("Validating file {} against category {}", f.path, cat);
                let rev = analyze_category(&content, &best_md, cat, args.debug).await?;
                if args.debug {
                    println!("(DEBUG) Category {} review:\n{}", cat, rev);
                }
                cat_reviews.push(rev);
            }

            let request = build_final_report_request(&cat_reviews);
            if args.debug {
                println!("(DEBUG) OpenAI request:\n{}", serde_json::to_string_pretty(&request)?);
            }

            let response = call_azure_openai(&request).await?;
            response.choices[0].message.content.clone()
        };

        if args.debug {
            println!("(DEBUG) Final response content:\n{}", response_content);
        }

        // Parse results and create comment
        let report = match serde_json::from_str::<FinalReport>(&response_content) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to parse JSON response: {}", e);
                eprintln!("Response content was:\n{}", response_content);
                return Err("JSON parsing failed".into());
            }
        };

        let findings = filter_by_severity(&report, args.minimum_severity);
        if !findings.is_empty() {
            let comment = format_pr_comment(&f.path, &findings);
            create_review_thread(
                &args.organization,
                &args.project,
                args.pull_request_id,
                &repo_id,
                &f.path,
                &comment,
                &args.pat,
            )
            .await?;
        }
    }

    Ok(())
}

/// Build an ADO comment for one file
fn format_pr_comment(file_path: &str, findings: &[&ValidationResult]) -> String {
    let mut out = format!("## Bicep Review Results for `{}`\n\n", file_path);
    for f in findings {
        let sev_emoji = match f.severity {
            5 => "🚨",
            4 => "⚠️",
            3 => "⚡",
            2 => "ℹ️",
            _ => "💡",
        };
        out.push_str(&format!(
            "### {emoji} Severity {sev}: {finding}\n**Impact:** {impact}\n\n",
            emoji = sev_emoji,
            sev = f.severity,
            finding = f.finding,
            impact = f.impact
        ));
    }
    out
}
