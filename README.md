<div align="center">
  <h1>🎯 bicep-analyzer</h1>
  
  <p align="center">
    <strong>Advanced Azure Bicep code analysis powered by AI</strong>
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

bicep-analyzer is a powerful command-line tool that leverages AI to analyze Azure Bicep Infrastructure as Code files. It provides intelligent insights, best practice recommendations, and security findings to help you write better Bicep code.

- 🤖 **AI-Powered Analysis**: Uses Azure OpenAI to understand and evaluate your code
- 🎯 **Category-Focused Analysis**: Improves accuracy by having the LLM analyze one aspect at a time:
  - Parameters
  - Variables
  - Naming
  - Resources
  - Outputs
  
  This focused approach allows the AI to concentrate deeply on specific aspects rather than trying to analyze everything simultaneously, resulting in more thorough and accurate findings.
- ⚡ **Intelligent Search**: Utilizes Azure AI Search for finding and learning from relevant code examples (Currently dummy files)
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

## ✨ Key Features

### 🔍 Smart Analysis
- **Category-Based Processing**: Analyzes code in focused categories for improved accuracy
- **Best Practices Validation**: Checks against established Bicep coding standards
- **Azure Context Awareness**: Understands Azure-specific resources and patterns
- **Code Example Learning**: Uses real-world examples to improve recommendations

### 📊 Comprehensive Reporting
- **Severity Levels**: Clear 1-5 rating system:
  - 5: Critical issues requiring immediate attention
  - 4: Serious concerns that should be addressed
  - 3: Important improvements recommended
  - 2: Minor suggestions for better code
  - 1: Optional optimizations
- **Detailed Impact Analysis**: Explains the consequences of each finding
- **Actionable Recommendations**: Clear guidance on how to improve the code

### ⚙️ Flexible Configuration
- **Category Selection**: Analyze specific aspects of your code
- **Severity Thresholds**: Filter findings by minimum severity
- **Debug Mode**: Detailed insight into the analysis process
- **Custom Best Practices**: Support for organization-specific standards

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

```bash
bicep-analyzer \
  --bicep-file <path-to-bicep> \
  --best-practices-file <path-to-md> \
  [--category <specific-category>] \
  [--minimum-severity <1-5>] \
  [--debug]
```

### Command Line Arguments

| Argument | Description | Required |
|----------|-------------|----------|
| `--bicep-file` | Path to the Bicep file for analysis | Yes |
| `--best-practices-file` | Path to markdown file containing best practices | Yes |
| `--category` | Optional specific category to analyze | No |
| `--minimum-severity` | Minimum severity level (1-5) to include in results | No |
| `--debug` | Enable debug output for LLM requests/responses | No |

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

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Azure OpenAI for providing the AI capabilities
- Azure AI Search for enabling intelligent code example search

---

<div align="center">
  Made with ❤️ for the Infrastructure as Code community
</div>
