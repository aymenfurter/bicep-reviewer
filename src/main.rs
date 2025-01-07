mod models;
mod services;

#[macro_use] 
extern crate prettytable;

use crate::models::*;
use crate::services::*;
use clap::Parser;
use std::{fs, process};
use prettytable::{Table, Row, Cell};

const TABLE_WRAP_WIDTH: usize = 60;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = initialize_application()?;
    let reviews = analyze_bicep_file(&config).await?;
    let report = generate_final_report(reviews, config.minimum_severity).await?;
    
    println!("\n{}", report);

    if report.contains("5 (Critical)") {
        process::exit(1);
    }

    Ok(())
}

struct AppConfig {
    bicep_content: String,
    best_practices_content: String,
    categories: Vec<String>,
    minimum_severity: u8,
    debug: bool,
}

fn initialize_application() -> Result<AppConfig, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let args = Args::parse();
    
    validate_arguments(&args)?;
    check_environment_variables()?;
    set_debug_flag(args.debug);

    let bicep_content = fs::read_to_string(&args.bicep_file)?;
    let best_practices_content = fs::read_to_string(&args.best_practices_file)?;
    let categories = determine_categories(args.category);

    Ok(AppConfig {
        bicep_content,
        best_practices_content,
        categories,
        minimum_severity: args.minimum_severity,
        debug: args.debug,
    })
}

fn validate_arguments(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.minimum_severity < 1 || args.minimum_severity > 5 {
        return Err("Minimum severity must be between 1 and 5".into());
    }
    Ok(())
}

fn set_debug_flag(debug: bool) {
    if debug {
        std::env::set_var("BICEP_DEBUG", "true");
    }
}

fn determine_categories(category_option: Option<String>) -> Vec<String> {
    match category_option {
        Some(category) => vec![category],
        None => DEFAULT_CATEGORIES.iter().map(|&s| s.to_string()).collect()
    }
}

fn check_environment_variables() -> Result<(), Box<dyn std::error::Error>> {
    let required_vars = get_required_environment_variables();
    let missing = find_missing_variables(&required_vars);

    if !missing.is_empty() {
        return Err(format!(
            "Missing required environment variables:\n{}", 
            missing.join("\n")
        ).into());
    }

    Ok(())
}

fn get_required_environment_variables() -> Vec<(&'static str, &'static str)> {
    vec![
        ("AZURE_OPENAI_ENDPOINT", "Azure OpenAI endpoint URL"),
        ("AZURE_OPENAI_API_KEY", "Azure OpenAI API key"),
        ("AZURE_OPENAI_DEPLOYMENT", "Azure OpenAI deployment name"),
        ("AZURE_SEARCH_ENDPOINT", "Azure Search endpoint URL"),
        ("AZURE_SEARCH_ADMIN_KEY", "Azure Search admin key"),
        ("AZURE_SEARCH_INDEX", "Azure Search index name"),
    ]
}

fn find_missing_variables(required_vars: &[(&str, &str)]) -> Vec<String> {
    required_vars
        .iter()
        .filter(|(var, _)| std::env::var(var).is_err())
        .map(|(var, description)| format!("{} ({})", var, description))
        .collect()
}

async fn analyze_bicep_file(config: &AppConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut all_reviews = Vec::new();

    for category in &config.categories {
        if config.debug {
            println!("\n=== Analyzing {} ===", category);
        }

        let review = analyze_category(
            &config.bicep_content,
            &config.best_practices_content,
            category,
            config.debug
        ).await?;

        all_reviews.push(review);
    }

    Ok(all_reviews)
}

async fn analyze_category(
    bicep_content: &str,
    best_practices_content: &str,
    category: &str,
    debug: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let practices = generate_category_practices(best_practices_content, category).await?;

    if debug {
        println!("Found {} best practices for {}", practices.len(), category);
    }

    let samples = query_azure_search(category).await?;
    if debug {
        println!("Found {} samples for {}", samples.len(), category);
        println!("Examples:\n{}", samples.join("\n"));
    }
    validate_category(bicep_content, category, &practices, &samples).await
}

async fn generate_final_report(review_texts: Vec<String>, min_severity: u8) -> Result<String, Box<dyn std::error::Error>> {
    let request = create_report_request(&review_texts);
    let response = call_azure_openai(&request).await?;
    let report: FinalReport = serde_json::from_str(&response.choices[0].message.content)?;
    
    let findings = filter_and_sort_findings(&report, min_severity);
    format_report(&findings, min_severity)
}

fn create_report_request(review_texts: &[String]) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: std::env::var("AZURE_OPENAI_DEPLOYMENT").unwrap_or_default(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: create_report_prompt(review_texts),
        }],
        temperature: 0.3,
        response_format: ResponseFormat {
            format_type: "json_object".to_string(),
        },
    }
}

fn create_report_prompt(review_texts: &[String]) -> String {
    format!(
        "Convert these review findings into structured data. Extract category, finding, severity, and impact.\n\
         Combine any duplicate findings and sort by severity.\n\
         Any nitpicks or minor suggestions can be removed.\
         Return in this exact JSON format:\n\
         {{\n  \"findings\": [\n    {{\n\
             \"category\": \"Category name\",\n\
             \"finding\": \"Description of the issue\",\n\
             \"severity\": 1-5,\n\
             \"impact\": \"Description of the impact\"\n    }}\n  ]\n}}\n\n\
         Reviews to process:\n{}", 
        review_texts.join("\n\n")
    )
}

fn filter_and_sort_findings(report: &FinalReport, min_severity: u8) -> Vec<&ValidationResult> {
    let mut findings: Vec<_> = report.findings
        .iter()
        .filter(|f| f.severity >= min_severity)
        .collect();
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}

fn format_report(findings: &[&ValidationResult], min_severity: u8) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();
    output.push_str("# Bicep Code Review Results\n\n");
    
    add_summary(&mut output, findings, min_severity);
    add_findings_table(&mut output, findings)?;
    add_critical_warning(&mut output, findings);

    Ok(output)
}

fn add_summary(output: &mut String, findings: &[&ValidationResult], min_severity: u8) {
    if !findings.is_empty() {
        output.push_str(&format!(
            "Found {} issues with severity {} or higher to address.\n\n", 
            findings.len(),
            min_severity
        ));
    } else {
        output.push_str(&format!(
            "No issues found with severity {} or higher.\n\n",
            min_severity
        ));
    }
}

fn add_findings_table(output: &mut String, findings: &[&ValidationResult]) -> Result<(), Box<dyn std::error::Error>> {
    let mut table = Table::new();
    table.add_row(row![b => "Category", "Finding", "Severity", "Impact"]);
    
    for finding in findings {
        add_finding_to_table(&mut table, finding);
    }

    let table_string = format_table_string(&table);
    output.push_str("```\n");
    output.push_str(&table_string);
    output.push_str("\n```\n");

    Ok(())
}

fn add_finding_to_table(table: &mut Table, finding: &ValidationResult) {
    let severity_label = get_severity_label(finding.severity);
    let wrapped_finding = textwrap::fill(&finding.finding, TABLE_WRAP_WIDTH);
    let wrapped_impact = textwrap::fill(&finding.impact, TABLE_WRAP_WIDTH);

    table.add_row(Row::new(vec![
        Cell::new(&finding.category),
        Cell::new(&wrapped_finding),
        Cell::new(&severity_label),
        Cell::new(&wrapped_impact),
    ]));
}

fn get_severity_label(severity: u8) -> &'static str {
    match severity {
        5 => "5 (Critical) ⚠️",
        4 => "4 (Serious)",
        3 => "3 (Important)",
        2 => "2 (Minor)",
        _ => "1 (Suggestion)",
    }
}

fn format_table_string(table: &Table) -> String {
    table.to_string()
        .lines()
        .map(|line| format!("    {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn add_critical_warning(output: &mut String, findings: &[&ValidationResult]) {
    if findings.iter().any(|f| f.severity == 5) {
        output.push_str("\n⚠️ **CRITICAL ISSUES FOUND!** These must be addressed before deployment.\n");
    }
}