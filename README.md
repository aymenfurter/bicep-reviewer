<div align="center">
  <h1>🎯 bicep-analyzer</h1>
  
  <p align="center">
    <strong>Azure Bicep code analysis powered by AI</strong>
  </p>
  <p align="center">
    <a href="#-quickstart">Quickstart</a> •
    <a href="#-key-features">Features</a> •
    <a href="#-installation">Installation</a> •
    <a href="#%EF%B8%8F-usage">Usage</a>
  </p>
  <p align="center">
    <img alt="License" src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge">
    <img alt="Rust" src="https://img.shields.io/badge/rust-stable-orange?style=for-the-badge">
    <img alt="Azure" src="https://img.shields.io/badge/azure-ready-0078D4?style=for-the-badge">
  </p>
</div>

## 🌟 What is bicep-analyzer?

bicep-analyzer is a command-line tool that leverages AI to analyze Azure Bicep Infrastructure as Code files. It provides intelligent insights, best practice recommendations, and security findings to help you write better Bicep code. This codebase demonstrates how to construct intelligent, LLM-derived validations for coding artifacts, directly integrating them into a CI/CD workflow.

- 🤖 **AI-Powered Analysis**: Uses Azure OpenAI to understand and evaluate your code
- 🎯 **Category-Focused Analysis**: Improves accuracy by having the LLM analyze one aspect at a time:
  - Parameters
  - Variables
  - Naming
  - Resources
  - Outputs
  
  This focused approach allows the AI to concentrate deeply on specific aspects rather than trying to analyze everything simultaneously, resulting in more thorough and accurate findings.
  
- ⚡ **Intelligent Search**: Utilizes Azure AI Search for finding and learning from relevant code examples (Currently the search lookup is not yet fully implemented)
- 📊 **Severity-Based Findings**: Clear categorization of issues from Critical (5) to Suggestions (1)

## 🚀 Quickstart

1. Set required environment variables:
```bash
export AZURE_OPENAI_ENDPOINT="your-endpoint"
export AZURE_OPENAI_API_KEY="your-key"
export AZURE_OPENAI_DEPLOYMENT="deployment-name"
export AZURE_SEARCH_ENDPOINT="search-endpoint"
export AZURE_SEARCH_ADMIN_KEY="search-key"
export AZURE_SEARCH_INDEX="index-name"
```

2. Run the analyzer:
```bash
bicep-analyzer --bicep-file main.bicep --best-practices-file practices.md
```

## 📝 Example

Here's a sample Bicep file with several common issues:

```bicep
param skuName string = 'Standard_LRS'
resource storageaccountName 'Microsoft.Storage/storageAccounts@2022-09-01' = {
  // Notice symbolic name is not lowerCamelCase
  // Param name is also not descriptive
  name: 'MyStorage${uniqueString(resourceGroup().id)}'
  location: resourceGroup().location
  sku: {
    name: skuName
  }
  kind: 'StorageV2'
}
output stgName string = storageaccountName.name
```

Running the analyzer produces this detailed report:

```markdown
# Bicep Code Review Results

Found 8 issues with severity 3 or higher to address.

+------------+--------------------------------------------------------------+---------------+--------------------------------------------------------------+
| Category   | Finding                                                      | Severity      | Impact                                                       |
+------------+--------------------------------------------------------------+---------------+--------------------------------------------------------------+
| Parameters | Parameter Name Not Descriptive: The parameter `skuName` is   | 3 (Important) | This can lead to confusion for collaborators and             |
|            | not descriptive enough.                                      |               | maintainers, reducing the clarity and readability of the     |
|            |                                                              |               | code.                                                        |
+------------+--------------------------------------------------------------+---------------+--------------------------------------------------------------+
| Parameters | Parameter Description Missing: The parameter `skuName` lacks | 3 (Important) | Without a description, users lack guidance and context about |
|            | a description.                                               |               | the parameter's purpose and usage.                           |
+------------+--------------------------------------------------------------+---------------+--------------------------------------------------------------+
```

See the following run as a full example: https://github.com/aymenfurter/bicep-reviewer/actions/runs/12654380705/job/35262121604

## ✨ Key Features

### 🔍 Smart Analysis
- **Category-Based Processing**: Analyzes code in focused categories for improved accuracy
- **Best Practices Validation**: Checks against established Bicep coding standards
- **Code Example Learning**: Uses code examples to improve recommendations

### 📊 Comprehensive Reporting
- **Severity Levels**: Clear 1-5 rating system
- **Detailed Impact Analysis**: Explains the consequences of each finding
- **Actionable Recommendations**: Clear guidance on how to improve the code

### ⚙️ Flexible Configuration
- **Category Selection**: Analyze specific aspects of your code
- **Severity Thresholds**: Filter findings by minimum severity
- **Debug Mode**: Detailed insight into the analysis process

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/aymenfurter/bicep-analyzer

# Build the project
cd bicep-analyzer
cargo build --release

# Run the binary
./target/release/bicep-analyzer --help
```

## 🛠️ Usage

### Local Analysis

```bash
bicep-analyzer \
  --bicep-file <path-to-bicep> \
  --best-practices-file <path-to-md> \
  [--category <specific-category>] \
  [--minimum-severity <1-5>] \
  [--simple] \
  [--debug]
```

### Azure DevOps Integration

The tool can be integrated into your Azure DevOps pull request workflow to automatically review Bicep files. Here's how to set it up:

1. Create a variable group named `bicep-reviewer-params` containing:
   ```
   AZURE_OPENAI_ENDPOINT
   AZURE_OPENAI_API_KEY
   AZURE_OPENAI_DEPLOYMENT
   AZURE_SEARCH_ENDPOINT
   AZURE_SEARCH_ADMIN_KEY
   AZURE_SEARCH_INDEX
   ```

2. Add this complete azure-pipelines.yml to your repository:
```
trigger: none

pr:
  branches:
    include:
      - main
  paths:
    include:
      - '**/*.bicep'

pool:
  vmImage: 'ubuntu-latest'

variables:
  - group: bicep-reviewer-params
  - name: CARGO_TERM_COLOR
    value: always
  - name: RUSTUP_TOOLCHAIN
    value: stable

steps:
- task: DownloadSecureFile@1
  inputs:
    secureFile: 'bicep_llm_validator'
  name: downloadBinary
  displayName: 'Download Bicep Validator Binary'

- task: DownloadSecureFile@1
  inputs:
    secureFile: 'bicep-best-practices.md'
  name: downloadRules
  displayName: 'Download Bicep Best Practices Rules'

- script: |
    chmod +x $(Agent.TempDirectory)/$(downloadBinary.secureFilePath)
  displayName: 'Make Binary Executable'

- script: |
    if [ -z "$(System.PullRequest.PullRequestId)" ]; then
      echo "Error: No PullRequest ID"
      exit 1
    fi

    BINARY_PATH="$(Agent.TempDirectory)/$(downloadBinary.secureFilePath)"
    RULES_FILE="$(Agent.TempDirectory)/$(downloadRules.secureFilePath)"

    ORG_NAME=$(echo "$(System.CollectionUri)" | sed -E 's@.*/dev\.azure\.com/([^/]+).*@\1@')

    "$BINARY_PATH" azure \
      --organization "$ORG_NAME" \
      --project "automation" \
      --pull-request-id "$(System.PullRequest.PullRequestId)" \
      --pat "$(ADO_PAT)" \
      --best-practices-file "$RULES_FILE" \
      --minimum-severity 3 \
      --repository "<your-repository-name>" \
      --simple
  env:
    ADO_PAT: $(ADO_PAT)
    AZURE_OPENAI_ENDPOINT: $(AZURE_OPENAI_ENDPOINT)
    AZURE_OPENAI_API_KEY: $(AZURE_OPENAI_API_KEY)
    AZURE_OPENAI_DEPLOYMENT: $(AZURE_OPENAI_DEPLOYMENT)
    AZURE_SEARCH_ENDPOINT: $(AZURE_SEARCH_ENDPOINT)
    AZURE_SEARCH_ADMIN_KEY: $(AZURE_SEARCH_ADMIN_KEY)
    AZURE_SEARCH_INDEX: $(AZURE_SEARCH_INDEX)
  displayName: 'Run Bicep Review'
```

3. Pipeline Setup Requirements:
   - Create a PAT (Personal Access Token) with Code (Read & Write) permissions
   - Add it as a pipeline variable named `ADO_PAT` (mark as secret)
   - Build and add a bicep-validator as a Secure File
   - Add your bicep rule set as a Secure File
   - Create a variable group containing Azure OpenAI and Search settings

4. Pipeline Features:
   - Automatically triggers on PRs containing .bicep files
   - Builds the analyzer from source
   - Uses organization name from ADO URL
   - Posts findings as PR comments
   - Supports both simple and detailed analysis modes
   - Configurable severity thresholds

## 🔧 Environment Setup

Required environment variables:

```bash
# Azure OpenAI Configuration
AZURE_OPENAI_ENDPOINT="https://your-endpoint.openai.azure.com"
AZURE_OPENAI_API_KEY="your-api-key"
AZURE_OPENAI_DEPLOYMENT="deployment-name"

# Azure AI Search Configuration
AZURE_SEARCH_ENDPOINT="https://your-search.search.windows.net"
AZURE_SEARCH_ADMIN_KEY="your-search-key"
AZURE_SEARCH_INDEX="your-index-name"
```

## 📝 License

The rust code is licensed under the MIT License.

## 🙏 Acknowledgments

- Azure OpenAI for providing the AI capabilities
- Azure AI Search for enabling intelligent code example search

---

<div align="center">
  Made with ❤️ for the Infrastructure as Code community
</div>
