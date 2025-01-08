use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::models::{PullRequestFile, Thread, ThreadComment, ThreadContext};

const DEFAULT_API_VERSION: &str = "2023-03-15-preview";
const DEFAULT_TEMPERATURE: f32 = 0.3;
const HIGH_TEMPERATURE: f32 = 0.7;

/// Represents a request to the Azure OpenAI API
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub response_format: ResponseFormat,
}

/// Specifies the format for API responses
#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

/// Represents a message in the chat completion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Response from the chat completion API
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

/// Individual choice from the API response
#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
}

/// Azure Search response structure
#[derive(Debug, Deserialize)]
pub struct SearchResults {
    pub value: Vec<SearchDoc>,
}

/// Individual document from search results
#[derive(Debug, Deserialize)]
pub struct SearchDoc {
    pub content: String,
}

/// Generates best practices for a specific category
pub async fn generate_category_practices(
    markdown: &str, 
    category: &str
) -> Result<Vec<String>, Box<dyn Error>> {
    let request = create_practices_request(markdown, category)?;
    let response = call_azure_openai(&request).await?;
    
    Ok(response.choices[0].message.content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect())
}

/// Creates the request for generating best practices
fn create_practices_request(
    markdown: &str, 
    category: &str
) -> Result<ChatCompletionRequest, Box<dyn Error>> {
    Ok(ChatCompletionRequest {
        model: std::env::var("AZURE_OPENAI_DEPLOYMENT")?,
        messages: vec![
            create_system_message(),
            create_user_message(markdown, category),
        ],
        temperature: HIGH_TEMPERATURE,
        response_format: ResponseFormat {
            format_type: "text".to_string(),
        },
    })
}

/// Creates the system message for the API request
fn create_system_message() -> ChatMessage {
    ChatMessage {
        role: "system".to_string(),
        content: "You are an expert in Bicep Infrastructure as Code best practices.".to_string(),
    }
}

/// Creates the user message for the API request
fn create_user_message(markdown: &str, category: &str) -> ChatMessage {
    ChatMessage {
        role: "user".to_string(),
        content: format!(
            "Given this Bicep best practices documentation:\n\n{}\n\n\
             Extract and enhance the best practices specifically for the '{}' category.\n\
             Focus on security, maintainability, and reliability.\n\
             Return as a numbered list, one practice per line.",
            markdown, category
        ),
    }
}

/// Validates a category against best practices
pub async fn validate_category(
    code: &str,
    category: &str,
    practices: &[String],
    samples: &[String]
) -> Result<String, Box<dyn Error>> {
    let request = create_validation_request(code, category, practices, samples)?;
    let response = call_azure_openai(&request).await?;
    Ok(format!("Category: {}\n{}", category, response.choices[0].message.content))
}

/// Creates the request for category validation
fn create_validation_request(
    code: &str,
    category: &str,
    practices: &[String],
    samples: &[String]
) -> Result<ChatCompletionRequest, Box<dyn Error>> {
    Ok(ChatCompletionRequest {
        model: std::env::var("AZURE_OPENAI_DEPLOYMENT")?,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a Bicep code reviewer. For each issue, provide severity (1-5) and explain the impact.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: create_validation_prompt(code, category, practices, samples),
            }
        ],
        temperature: DEFAULT_TEMPERATURE,
        response_format: ResponseFormat {
            format_type: "text".to_string(),
        },
    })
}

/// Creates the validation prompt for the API request
fn create_validation_prompt(
    code: &str,
    category: &str,
    practices: &[String],
    samples: &[String]
) -> String {
    format!(
        "Review this Bicep code against these best practices for the {} category.\n\
         Only review the code that is under control of the developer.\n\
         Rate each issue severity 1-5:\n\
         5: Critical/blocking issue that must be fixed\n\
         4: Serious issue with significant risks\n\
         3: Important issue to address\n\
         2: Minor improvement needed\n\
         1: Suggestion for better practices\n\n\
         Best Practices:\n{}\n\n\
         Reference Examples:\n{}\n\n\
         Code to Review:\n{}\n\n\
         For each issue found, describe:\n\
         - The issue and where it occurs\n\
         - Its severity (1-5)\n\
         - The potential impact",
        category,
        practices.join("\n"),
        samples.join("\n---\n"),
        code
    )
}

/// Queries Azure Search for relevant examples
pub async fn query_azure_search(category: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let search_config = get_search_config()?;
    let url = build_search_url(&search_config, category);
    let results = execute_search_request(&url, &search_config.key).await?;
    
    Ok(results.value.into_iter().map(|d| d.content).collect())
}

/// Configuration for Azure Search
struct SearchConfig {
    endpoint: String,
    key: String,
    index: String,
}

/// Retrieves search configuration from environment variables
fn get_search_config() -> Result<SearchConfig, Box<dyn Error>> {
    Ok(SearchConfig {
        endpoint: std::env::var("AZURE_SEARCH_ENDPOINT")?,
        key: std::env::var("AZURE_SEARCH_ADMIN_KEY")?,
        index: std::env::var("AZURE_SEARCH_INDEX")?,
    })
}

/// Builds the search URL
fn build_search_url(config: &SearchConfig, category: &str) -> String {
    format!(
        "{}/indexes/{}/docs?api-version=2021-04-30-Preview&search={}&$top=2",
        config.endpoint, config.index, category
    )
}

/// Executes the search request
async fn execute_search_request(url: &str, key: &str) -> Result<SearchResults, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("api-key", key)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(serde_json::from_str(&response)?)
}

/// Calls Azure OpenAI API
pub async fn call_azure_openai(
    request: &ChatCompletionRequest
) -> Result<ChatCompletionResponse, Box<dyn Error>> {
    let debug_enabled = is_debug_enabled();
    
    if debug_enabled {
        log_request(request)?;
    }

    let config = get_openai_config()?;
    let url = build_openai_url(&config);
    let response = execute_openai_request(&url, &config.api_key, request).await?;
    
    if debug_enabled {
        log_response(&response)?;
    }

    Ok(response)
}

/// Configuration for Azure OpenAI
struct OpenAIConfig {
    endpoint: String,
    api_key: String,
    deployment: String,
    api_version: String,
}

/// Retrieves OpenAI configuration from environment variables
fn get_openai_config() -> Result<OpenAIConfig, Box<dyn Error>> {
    Ok(OpenAIConfig {
        endpoint: std::env::var("AZURE_OPENAI_ENDPOINT")?,
        api_key: std::env::var("AZURE_OPENAI_API_KEY")?,
        deployment: std::env::var("AZURE_OPENAI_DEPLOYMENT")?,
        api_version: std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| DEFAULT_API_VERSION.to_string()),
    })
}

/// Builds the OpenAI API URL
fn build_openai_url(config: &OpenAIConfig) -> String {
    format!(
        "{}/openai/deployments/{}/chat/completions?api-version={}",
        config.endpoint, config.deployment, config.api_version
    )
}

/// Executes the OpenAI API request
async fn execute_openai_request(
    url: &str,
    api_key: &str,
    request: &ChatCompletionRequest
) -> Result<ChatCompletionResponse, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("api-key", api_key)
        .json(request)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

/// Checks if debug mode is enabled
fn is_debug_enabled() -> bool {
    std::env::var("BICEP_DEBUG").unwrap_or_else(|_| "false".to_string()) == "true"
}

/// Logs the API request for debugging
fn log_request(request: &ChatCompletionRequest) -> Result<(), Box<dyn Error>> {
    println!("\n=== LLM Request ===\n{}", serde_json::to_string_pretty(request)?);
    Ok(())
}

/// Logs the API response for debugging
fn log_response(response: &ChatCompletionResponse) -> Result<(), Box<dyn Error>> {
    println!("\n=== LLM Response ===\n{}", serde_json::to_string_pretty(response)?);
    Ok(())
}

/// Gets modified Bicep files from a pull request
pub async fn get_modified_bicep_files(
    org: &str,
    project: &str,
    pr_id: i32,
    pat: &str,
) -> Result<Vec<PullRequestFile>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}_apis/git/repositories/{}/pullRequests/{}/changes?api-version=6.0",
        org, project, pr_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Basic {}", base64::encode(format!(":{}", pat))))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    let files = response["changes"]
        .as_array()
        .ok_or("No changes found")?
        .iter()
        .filter(|change| {
            change["item"]["path"]
                .as_str()
                .map(|path| path.ends_with(".bicep"))
                .unwrap_or(false)
        })
        .map(|change| PullRequestFile {
            path: change["item"]["path"].as_str().unwrap().to_string(),
            change_type: change["changeType"].as_str().unwrap().to_string(),
        })
        .collect();

    Ok(files)
}

/// Gets file content from a pull request
pub async fn get_file_content(
    org: &str,
    project: &str,
    pr_id: i32,
    path: &str,
    pat: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}_apis/git/repositories/{}/pullRequests/{}/iterations/1/changes/{}/content?api-version=6.0",
        org, project, pr_id, path
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Basic {}", base64::encode(format!(":{}", pat))))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(response)
}

/// Creates a review comment thread in the pull request
pub async fn create_review_thread(
    org: &str,
    project: &str,
    pr_id: i32,
    file_path: &str,
    comment: &str,
    pat: &str,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}_apis/git/repositories/{}/pullRequests/{}/threads?api-version=6.0",
        org, project, pr_id
    );

    let thread = Thread {
        comments: vec![ThreadComment {
            content: comment.to_string(),
            comment_type: 1,
        }],
        status: 1,
        thread_context: ThreadContext {
            file_path: file_path.to_string(),
        },
    };

    client
        .post(&url)
        .header("Authorization", format!("Basic {}", base64::encode(format!(":{}", pat))))
        .json(&thread)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_system_message() {
        let message = create_system_message();
        assert_eq!(message.role, "system");
        assert!(!message.content.is_empty());
    }

    #[test]
    fn test_build_search_url() {
        let config = SearchConfig {
            endpoint: "https://example.com".to_string(),
            key: "key".to_string(),
            index: "index".to_string(),
        };
        let url = build_search_url(&config, "test");
        assert!(url.contains("https://example.com"));
        assert!(url.contains("index"));
        assert!(url.contains("test"));
    }
}