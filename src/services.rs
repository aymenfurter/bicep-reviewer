// services.rs

use crate::models::{PullRequestFile, Thread, ThreadComment, ThreadContext};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// ------------------------------------------------------------
/// Azure OpenAI

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub response_format: ResponseFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
}

const DEFAULT_API_VERSION: &str = "2023-03-15-preview";

struct OpenAIConfig {
    endpoint: String,
    api_key: String,
    deployment: String,
    api_version: String,
}

/// Call the Azure OpenAI chat
pub async fn call_azure_openai(
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, Box<dyn Error>> {
    if is_debug_enabled() {
        println!("(DEBUG) call_azure_openai - Request:\n{}", serde_json::to_string_pretty(request)?);
    }

    let cfg = get_openai_config()?;
    let url = format!(
        "{}/openai/deployments/{}/chat/completions?api-version={}",
        cfg.endpoint, cfg.deployment, cfg.api_version
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("api-key", &cfg.api_key)
        .json(request)
        .send()
        .await?
        .error_for_status()?;

    let json = resp.json::<ChatCompletionResponse>().await?;
    if is_debug_enabled() {
        println!("(DEBUG) call_azure_openai - Response:\n{}", serde_json::to_string_pretty(&json)?);
    }
    Ok(json)
}

fn get_openai_config() -> Result<OpenAIConfig, Box<dyn Error>> {
    Ok(OpenAIConfig {
        endpoint: std::env::var("AZURE_OPENAI_ENDPOINT")?,
        api_key: std::env::var("AZURE_OPENAI_API_KEY")?,
        deployment: std::env::var("AZURE_OPENAI_DEPLOYMENT")?,
        api_version: std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| DEFAULT_API_VERSION.to_string()),
    })
}

fn is_debug_enabled() -> bool {
    std::env::var("BICEP_DEBUG").unwrap_or_else(|_| "false".to_string()) == "true"
}

/// ------------------------------------------------------------
/// Bicep analysis function: analyze_category

/// This function pulls best practices, references, and calls validate
pub async fn analyze_category(
    bicep_code: &str,
    best_practices_md: &str,
    category: &str,
    debug: bool,
) -> Result<String, Box<dyn Error>> {
    if debug {
        println!("(DEBUG) analyze_category => starting analysis for '{}'", category);
        println!("(DEBUG) analyze_category => bicep code length: {}", bicep_code.len());
        println!("(DEBUG) analyze_category => best practices doc length: {}", best_practices_md.len());
    }

    let practices = match generate_category_practices(best_practices_md, category).await {
        Ok(p) => {
            if debug {
                println!("(DEBUG) analyze_category => found {} practices", p.len());
            }
            p
        }
        Err(e) => {
            eprintln!("(DEBUG) analyze_category => generate_category_practices failed: {}", e);
            return Err(e);
        }
    };

    if debug {
        println!("(DEBUG) analyze_category => getting references for '{}'", category);
    }

    let references = match query_azure_search(category).await {
        Ok(r) => {
            if debug {
                println!("(DEBUG) analyze_category => found {} references", r.len());
            }
            r
        }
        Err(e) => {
            eprintln!("(DEBUG) analyze_category => query_azure_search failed: {}", e);
            return Err(e);
        }
    };

    if debug {
        println!("(DEBUG) analyze_category => validating category");
    }

    match validate_category(bicep_code, category, &practices, &references).await {
        Ok(text) => {
            if debug {
                println!("(DEBUG) analyze_category => validation completed");
            }
            Ok(text)
        }
        Err(e) => {
            eprintln!("(DEBUG) analyze_category => validate_category failed: {}", e);
            Err(e)
        }
    }
}

/// Extract best practices lines from MD
pub async fn generate_category_practices(
    markdown: &str,
    category: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let deployment = match std::env::var("AZURE_OPENAI_DEPLOYMENT") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("(DEBUG) Failed to get AZURE_OPENAI_DEPLOYMENT: {}", e);
            return Err(Box::new(e));
        }
    };

    let req = ChatCompletionRequest {
        model: deployment.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an expert in Bicep IaC best practices.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Given this Bicep best practices doc:\n\n{}\n\n\
                     Extract best practices specifically for the '{}' category.\n\
                     Return as a list, one item per line.",
                    markdown, category
                ),
            },
        ],
        temperature: 0.7,
        response_format: ResponseFormat {
            format_type: "text".to_string(),
        },
    };

    match call_azure_openai(&req).await {
        Ok(resp) => {
            let lines: Vec<_> = resp.choices[0]
                .message
                .content
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            Ok(lines)
        }
        Err(e) => {
            eprintln!("(DEBUG) generate_category_practices => OpenAI call failed: {}", e);
            Err(e)
        }
    }
}

/// Validate the Bicep code snippet
pub async fn validate_category(
    code: &str,
    category: &str,
    practices: &[String],
    references: &[String],
) -> Result<String, Box<dyn Error>> {
    let deployment = match std::env::var("AZURE_OPENAI_DEPLOYMENT") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("(DEBUG) Failed to get AZURE_OPENAI_DEPLOYMENT: {}", e);
            return Err(Box::new(e));
        }
    };

    let req = ChatCompletionRequest {
        model: deployment,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a Bicep code reviewer. For each issue, provide severity (1-5) and impact."
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Review Bicep code for category '{}'.\n\
                     Severity scale:\n  5 = Critical\n  4 = Serious\n  3 = Important\n  2 = Minor\n  1 = Suggestion\n\n\
                     Best Practices:\n{}\n\n\
                     References:\n{}\n\n\
                     Code:\n{}\n\n\
                     For each issue:\n - The issue\n - Severity\n - Impact",
                    category,
                    practices.join("\n"),
                    references.join("\n---\n"),
                    code,
                ),
            },
        ],
        temperature: 0.3,
        response_format: ResponseFormat {
            format_type: "text".to_string(),
        },
    };

    let resp = call_azure_openai(&req).await?;
    Ok(format!("Category: {}\n{}", category, resp.choices[0].message.content))
}

/// Simple validation without categories
pub async fn validate_simple(
    code: &str,
    best_practices: &str,
) -> Result<String, Box<dyn Error>> {
    let deployment = std::env::var("AZURE_OPENAI_DEPLOYMENT")?;

    let req = ChatCompletionRequest {
        model: deployment,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a Bicep code reviewer. Review code against best practices and return findings in JSON format.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Review this Bicep code against these best practices.\n\
                     Return findings in this exact JSON format:\n\
                     {{\n  \"findings\": [\n    {{\n\
                     \"category\": \"General\",\n\"finding\": \"...\",\n\"severity\": 1-5,\n\"impact\": \"...\"\n    }}\n  ]\n}}\n\n\
                     Best Practices:\n{}\n\n\
                     Code to Review:\n{}\n\n\
                     Severity scale:\n\
                     5 = Critical security/reliability issues\n\
                     4 = Serious issues that should be fixed\n\
                     3 = Important improvements needed\n\
                     2 = Minor suggestions\n\
                     1 = Style/documentation suggestions",
                    best_practices,
                    code
                ),
            },
        ],
        temperature: 0.3,
        response_format: ResponseFormat {
            format_type: "json_object".to_string(),
        },
    };

    let resp = call_azure_openai(&req).await?;
    Ok(resp.choices[0].message.content.clone())
}

/// ------------------------------------------------------------
/// Azure Search references

#[derive(Debug, Deserialize)]
struct SearchResults {
    pub value: Vec<SearchDoc>,
}

#[derive(Debug, Deserialize)]
struct SearchDoc {
    pub content: String,
}

pub async fn query_azure_search(category: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let sc = get_search_config()?;
    let url = format!(
        "{}/indexes/{}/docs?api-version=2021-04-30-Preview&search={}&$top=2",
        sc.endpoint, sc.index, category
    );

    let client = reqwest::Client::new();
    let body = client
        .get(&url)
        .header("api-key", &sc.key)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let parsed: SearchResults = serde_json::from_str(&body)?;
    Ok(parsed.value.into_iter().map(|doc| doc.content).collect())
}

struct SearchConfig {
    endpoint: String,
    key: String,
    index: String,
}

fn get_search_config() -> Result<SearchConfig, Box<dyn Error>> {
    Ok(SearchConfig {
        endpoint: std::env::var("AZURE_SEARCH_ENDPOINT")?,
        key: std::env::var("AZURE_SEARCH_ADMIN_KEY")?,
        index: std::env::var("AZURE_SEARCH_INDEX")?,
    })
}

/// ------------------------------------------------------------
/// Azure DevOps: Repo ID, Listing changed files, Creating comments

#[derive(Debug, Deserialize)]
struct PullRequestIteration {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct PullRequestIterationList {
    value: Vec<PullRequestIteration>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PullRequestIterationChanges {
    #[serde(rename = "changeEntries")]
    change_entries: Vec<IterationChangeEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IterationChangeEntry {
    #[serde(rename = "changeTrackingId")]
    change_tracking_id: i32,
    #[serde(rename = "changeId")]
    change_id: i32,
    item: Option<IterationItem>,
    #[serde(rename = "changeType")]
    change_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IterationItem {
    #[serde(rename = "objectId")]
    object_id: Option<String>,
    #[serde(rename = "originalObjectId")]
    original_object_id: Option<String>,
    path: Option<String>,
}

/// Retrieve the GUID of the repository from its name
pub async fn get_repository_id(
    org: &str,
    project: &str,
    repo_name: &str,
    pat: &str,
) -> Result<String, Box<dyn Error>> {
    let debug_enabled = is_debug_enabled();
    let client = reqwest::Client::new();

    // Construct final org URL
    let org_url = if org.starts_with("https://dev.azure.com") {
        org.trim_end_matches('/').to_string()
    } else if org.contains("dev.azure.com") {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    } else {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    };

    let proj_enc = urlencoding::encode(project);
    let repo_enc = urlencoding::encode(repo_name);

    let url = format!(
        "{}/{}/_apis/git/repositories/{}?api-version=7.1",
        org_url, proj_enc, repo_enc
    );

    if debug_enabled {
        println!("(DEBUG) get_repository_id => {}", url);
    }

    let resp = client
        .get(&url)
        .header("Authorization", format!("Basic {}", BASE64.encode(format!(":{}", pat))))
        .send()
        .await?;

    if !resp.status().is_success() {
        let st = resp.status();
        let body = resp.text().await?;
        if debug_enabled {
            eprintln!("(DEBUG) get_repository_id => error: status={}, body={}", st, body);
        }
        return Err(format!("Repository request failed: status={}, body={}", st, body).into());
    }

    let repo_info = resp.json::<serde_json::Value>().await?;
    Ok(repo_info["id"].as_str().unwrap_or(repo_name).to_string())
}

/// List changed Bicep files from the latest iteration
pub async fn list_modified_bicep_files(
    org: &str,
    project: &str,
    repo_id: &str,
    pr_id: i32,
    pat: &str,
    debug: bool,
) -> Result<Vec<PullRequestFile>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let org_url = if org.starts_with("https://dev.azure.com") {
        org.trim_end_matches('/').to_string()
    } else if org.contains("dev.azure.com") {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    } else {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    };

    let proj_enc = urlencoding::encode(project);

    // 1) List PR iterations
    let iter_url = format!(
        "{}/{}/_apis/git/repositories/{}/pullRequests/{}/iterations?api-version=7.1",
        org_url, proj_enc, repo_id, pr_id
    );
    if debug {
        println!("(DEBUG) list_modified_bicep_files => iter_url = {}", iter_url);
    }

    let iter_resp = client
        .get(&iter_url)
        .header("Authorization", format!("Basic {}", BASE64.encode(format!(":{}", pat))))
        .send()
        .await?;

    if !iter_resp.status().is_success() {
        let st = iter_resp.status();
        let body = iter_resp.text().await?;
        if debug {
            eprintln!("(DEBUG) list_modified_bicep_files => iterations error: {} => {}", st, body);
        }
        return Err(format!("PullRequest iterations API error: status={}, body={}", st, body).into());
    }

    let iteration_list = iter_resp.json::<PullRequestIterationList>().await?;
    if iteration_list.value.is_empty() {
        if debug {
            eprintln!("(DEBUG) PR has no iterations => no changes");
        }
        return Ok(vec![]);
    }

    let latest_iter_id = iteration_list.value.iter().map(|x| x.id).max().unwrap_or(1);

    // 2) Get iteration changes
    let changes_url = format!(
        "{}/{}/_apis/git/repositories/{}/pullRequests/{}/iterations/{}/changes?api-version=7.1",
        org_url, proj_enc, repo_id, pr_id, latest_iter_id
    );
    if debug {
        println!("(DEBUG) list_modified_bicep_files => changes_url = {}", changes_url);
    }

    let changes_resp = client
        .get(&changes_url)
        .header("Authorization", format!("Basic {}", BASE64.encode(format!(":{}", pat))))
        .send()
        .await?;

    if !changes_resp.status().is_success() {
        let st = changes_resp.status();
        let body = changes_resp.text().await?;
        if debug {
            eprintln!("(DEBUG) iteration changes => error: {} => {}", st, body);
        }
        return Err(format!("Iteration changes API error: status={}, body={}", st, body).into());
    }

    let iteration_changes = changes_resp.json::<PullRequestIterationChanges>().await?;
    
    if debug {
        println!("(DEBUG) Raw changes response: {}", serde_json::to_string_pretty(&iteration_changes)?);
    }
    
    let mut results = Vec::new();
    for entry in iteration_changes.change_entries {
        if debug {
            println!("(DEBUG) Processing change entry {}: {:?}", entry.change_tracking_id, entry);
        }
        
        if let Some(item) = entry.item {
            if let Some(path) = item.path {
                if debug {
                    println!("(DEBUG) Found changed file: {} (objectId: {:?}, originalObjectId: {:?})", 
                        path, item.object_id, item.original_object_id);
                }
                
                if path.ends_with(".bicep") {
                    if debug {
                        println!("(DEBUG) Adding .bicep file: {}", path);
                    }
                    
                    results.push(PullRequestFile {
                        path,
                        change_type: entry.change_type.unwrap_or_else(|| "edit".to_string()),
                        object_id: item.object_id.unwrap_or_default(),
                        original_object_id: item.original_object_id,
                    });
                }
            }
        }
    }

    if debug {
        println!("(DEBUG) Found {} changed .bicep files in iteration {}", 
            results.len(), latest_iter_id);
        for file in &results {
            println!("(DEBUG) - {} ({} / {})", file.path, file.change_type, file.object_id);
        }
    }

    Ok(results)
}

/// Retrieve file content using object ID
pub async fn get_file_content(
    org: &str,
    project: &str,
    repo_id: &str,
    _pr_id: i32,  // Added underscore prefix to unused parameter
    path: &str,
    object_id: &str,
    pat: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let debug_enabled = is_debug_enabled();

    let org_url = if org.starts_with("https://dev.azure.com") {
        org.trim_end_matches('/').to_string()
    } else if org.contains("dev.azure.com") {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    } else {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    };

    let proj_enc = urlencoding::encode(project);
    let path_enc = urlencoding::encode(path);
    
    let url = format!(
        "{}/{}/_apis/git/repositories/{}/items?objectId={}&path={}&includeContent=true&api-version=7.1",
        org_url, proj_enc, repo_id, object_id, path_enc
    );
    if debug_enabled {
        println!("(DEBUG) get_file_content => {}", url);
    }

    let resp = client
        .get(&url)
        .header("Authorization", format!("Basic {}", BASE64.encode(format!(":{}", pat))))
        .header("Accept", "text/plain") // Explicitly request text
        .send()
        .await?;

    if !resp.status().is_success() {
        let st = resp.status();
        let body = resp.text().await?;
        if debug_enabled {
            eprintln!("(DEBUG) get_file_content => error: {} => {}", st, body);
        }
        return Err(format!("File content API error: status={}, body={}", st, body).into());
    }

    // Get the content directly as text
    let content = resp.text().await?;
    if debug_enabled {
        println!("(DEBUG) get_file_content => received {} bytes", content.len());
    }
    
    Ok(content)
}

/// Create a top-level thread in the PR
pub async fn create_review_thread(
    org: &str,
    project: &str,
    pr_id: i32,
    repo_id: &str,
    file_path: &str,
    comment: &str,
    pat: &str,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let debug_enabled = is_debug_enabled();

    let org_url = if org.starts_with("https://dev.azure.com") {

        org.trim_end_matches('/').to_string()
    } else if org.contains("dev.azure.com") {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    } else {
        format!("https://dev.azure.com/{}", org.trim_matches('/'))
    };

    let proj_enc = urlencoding::encode(project);
    let url = format!(
        "{}/{}/_apis/git/repositories/{}/pullRequests/{}/threads?api-version=7.1",
        org_url, proj_enc, repo_id, pr_id
    );
    if debug_enabled {
        println!("(DEBUG) create_review_thread => {}", url);
    }

    let thread = Thread {
        comments: vec![ThreadComment {
            content: comment.to_string(),
            comment_type: 1,
        }],
        status: 1, // active
        thread_context: ThreadContext {
            file_path: file_path.to_string(),
        },
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Basic {}", BASE64.encode(format!(":{}", pat))))
        .json(&thread)
        .send()
        .await?;

    if !resp.status().is_success() {
        let st = resp.status();
        let body = resp.text().await?;
        if debug_enabled {
            eprintln!("(DEBUG) create_review_thread => error: {} => {}", st, body);
        }
        return Err(format!("Create thread API error: status={}, body={}", st, body).into());
    }

    Ok(())
}
