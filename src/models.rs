use clap::Parser;
use serde::{Deserialize, Serialize};

// Constants
pub const DEFAULT_MIN_SEVERITY: u8 = 3;

pub const DEFAULT_CATEGORIES: [&str; 5] = [
    "Parameters",
    "Variables",
    "Naming",
    "Resources",
    "Outputs"
];

/// Command line arguments for the application
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Path to the Bicep file for analysis
    #[arg(long)]
    pub bicep_file: String,

    /// Path to markdown file containing best practices
    #[arg(long)]
    pub best_practices_file: String,

    /// Optional specific category to analyze
    /// If not provided, all categories will be analyzed
    #[arg(long)]
    pub category: Option<String>,

    /// Enable debug output for LLM requests/responses
    #[arg(long)]
    pub debug: bool,

    /// Minimum severity level (1-5) to include in results
    /// 5: Critical, 4: Serious, 3: Important, 2: Minor, 1: Suggestion
    #[arg(long, default_value_t = DEFAULT_MIN_SEVERITY)]
    pub minimum_severity: u8,
}

/// Azure DevOps specific command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct AzureDevOpsArgs {
    /// Azure DevOps organization URL
    #[arg(long)]
    pub organization: String,

    /// Azure DevOps project
    #[arg(long)]
    pub project: String,

    /// Pull Request ID
    #[arg(long)]
    pub pull_request_id: i32,

    /// Azure DevOps Personal Access Token
    #[arg(long)]
    pub pat: String,

    /// Path to markdown file containing best practices
    #[arg(long)]
    pub best_practices_file: String,

    /// Enable debug output for LLM requests/responses
    #[arg(long)]
    pub debug: bool,

    /// Minimum severity level (1-5) to include in results
    #[arg(long, default_value_t = DEFAULT_MIN_SEVERITY)]
    pub minimum_severity: u8,
}

/// Represents a single validation finding from the code review
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Category of the finding (e.g., "Parameters", "Variables")
    pub category: String,
    
    /// Description of the issue found
    pub finding: String,
    
    /// Severity level (1-5) of the finding
    #[serde(deserialize_with = "deserialize_severity")]
    pub severity: u8,
    
    /// Description of the potential impact of this issue
    pub impact: String,
}

/// Collection of best practices
#[derive(Debug, Serialize, Deserialize)]
pub struct BestPracticesResponse {
    pub practices: Vec<String>,
}

/// Collection of validation findings
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub findings: Vec<ValidationResult>,
}

/// Final report containing all findings
#[derive(Debug, Serialize, Deserialize)]
pub struct FinalReport {
    pub findings: Vec<ValidationResult>,
}

/// Custom deserializer for severity values
/// Handles both numeric and string inputs
pub fn deserialize_severity<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SeverityValue {
        Number(u8),
        String(()),
    }

    match SeverityValue::deserialize(deserializer)? {
        SeverityValue::Number(n) => Ok(n),
        SeverityValue::String(_) => Ok(0),
    }
}

#[derive(Debug, Deserialize)]
pub struct PullRequestFile {
    pub path: String,
    pub change_type: String,
}

#[derive(Debug, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ThreadComment {
    pub content: String,
    pub comment_type: i32,
}

#[derive(Debug, Serialize)]
pub struct Thread {
    pub comments: Vec<ThreadComment>,
    pub status: i32,
    pub thread_context: ThreadContext,
}

#[derive(Debug, Serialize)]
pub struct ThreadContext {
    pub file_path: String,
}