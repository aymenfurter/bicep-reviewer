// models.rs

use clap::Parser;
use serde::{Deserialize, Serialize};

/// Default min severity
pub const DEFAULT_MIN_SEVERITY: u8 = 3;

/// Default categories
pub const DEFAULT_CATEGORIES: [&str; 5] = [
    "Parameters",
    "Variables",
    "Naming",
    "Resources",
    "Outputs",
];

/// Local usage
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Path to local Bicep file
    #[arg(long)]
    pub bicep_file: String,

    /// Path to best practices MD
    #[arg(long)]
    pub best_practices_file: String,

    /// Single optional category
    #[arg(long)]
    pub category: Option<String>,

    /// Debug
    #[arg(long)]
    pub debug: bool,

    /// Minimum severity
    #[arg(long, default_value_t = DEFAULT_MIN_SEVERITY)]
    pub minimum_severity: u8,

    /// Simple mode - single prompt without categories
    #[arg(long)]
    pub simple: bool,
}

/// Azure DevOps usage
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct AzureDevOpsArgs {
    /// Org name or snippet
    #[arg(long)]
    pub organization: String,

    /// Azure DevOps project
    #[arg(long)]
    pub project: String,

    /// Pull Request ID
    #[arg(long)]
    pub pull_request_id: i32,

    /// PAT with code read/write
    #[arg(long)]
    pub pat: String,

    /// Best practices MD
    #[arg(long)]
    pub best_practices_file: String,

    /// Debug
    #[arg(long)]
    pub debug: bool,

    /// Minimum severity
    #[arg(long, default_value_t = DEFAULT_MIN_SEVERITY)]
    pub minimum_severity: u8,

    /// Human-friendly repo name
    #[arg(long)]
    pub repository: String,

    /// Simple mode - single prompt without categories
    #[arg(long)]
    pub simple: bool,
}

/// Validation result
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub category: String,
    pub finding: String,
    #[serde(deserialize_with = "deserialize_severity")]
    pub severity: u8,
    pub impact: String,
}

/// Final aggregated JSON
#[derive(Debug, Serialize, Deserialize)]
pub struct FinalReport {
    pub findings: Vec<ValidationResult>,
}

/// Custom deserializer for severity
pub fn deserialize_severity<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MaybeNum {
        Num(u8),
        #[allow(dead_code)]  // Allow dead code for serde deserialization
        Str(String),
    }

    match MaybeNum::deserialize(deserializer)? {
        MaybeNum::Num(n) => Ok(n),
        MaybeNum::Str(_) => Ok(0),
    }
}

/// Changed file in PR
#[derive(Debug, Deserialize)]
pub struct PullRequestFile {
    pub path: String,
    #[serde(rename = "changeType")]
    pub change_type: String,
    #[serde(rename = "objectId")]
    pub object_id: String,
    #[serde(rename = "originalObjectId")]
    #[allow(dead_code)]  // Allow dead code as it's part of the API response
    pub original_object_id: Option<String>,
}

/// Thread creation
#[derive(Debug, Serialize)]
pub struct Thread {
    pub comments: Vec<ThreadComment>,
    pub status: i32,
    pub thread_context: ThreadContext,
}

/// Single comment
#[derive(Debug, Serialize)]
pub struct ThreadComment {
    pub content: String,
    pub comment_type: i32,
}

/// Thread context (file path etc.)
#[derive(Debug, Serialize)]
pub struct ThreadContext {
    pub file_path: String,
}
