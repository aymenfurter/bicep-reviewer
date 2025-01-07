#!/usr/bin/env bash
set -euo pipefail

# Env variables needed:
#   AZURE_SEARCH_ENDPOINT: e.g. https://my-cog-search.search.windows.net
#   AZURE_SEARCH_ADMIN_KEY: Admin key for Azure AI Search
#   AZURE_SEARCH_INDEX: e.g. bicep-fewshot
#   CURATED_EXAMPLES_DIR: Local path to curated-examples/

# Default to curated-examples if not set
CURATED_EXAMPLES_DIR=${CURATED_EXAMPLES_DIR:-"./curated-examples"}

# Validate environment and directories
if [ -z "$AZURE_SEARCH_ENDPOINT" ] || [ -z "$AZURE_SEARCH_ADMIN_KEY" ] || [ -z "$AZURE_SEARCH_INDEX" ]; then
    echo "Missing required environment variables."
    exit 1
fi

if [ ! -d "$CURATED_EXAMPLES_DIR" ]; then
    echo "Error: Directory $CURATED_EXAMPLES_DIR does not exist"
    exit 1
fi

# Count bicep files
BICEP_FILES=("$CURATED_EXAMPLES_DIR"/*.bicep)
BICEP_COUNT=${#BICEP_FILES[@]}

if [ "$BICEP_COUNT" -eq 0 ]; then
    echo "Error: No .bicep files found in $CURATED_EXAMPLES_DIR"
    exit 1
fi

echo "Found $BICEP_COUNT .bicep files to process"

echo "Creating/Updating Azure AI Search index: $AZURE_SEARCH_INDEX"

# Create or update index
curl -X PUT \
    "$AZURE_SEARCH_ENDPOINT/indexes/$AZURE_SEARCH_INDEX?api-version=2021-04-30-Preview" \
    -H "Content-Type: application/json" \
    -H "api-key: $AZURE_SEARCH_ADMIN_KEY" \
    -d '{
    "name": "'"$AZURE_SEARCH_INDEX"'",
    "fields": [
        { "name": "id", "type": "Edm.String", "key": true, "searchable": false },
        { "name": "content", "type": "Edm.String", "searchable": true }
    ]
}'

echo -e "\nUploading curated examples from $CURATED_EXAMPLES_DIR"

# Build documents array
DOCS_JSON='{"value":['
FIRST_DOC=true

for f in "$CURATED_EXAMPLES_DIR"/*.bicep; do
    if [ -f "$f" ]; then
        # Create a valid document key by removing .bicep and replacing dots with underscores
        filename=$(basename "$f")
        docKey=$(echo "${filename%.*}" | sed 's/[^a-zA-Z0-9_-]/_/g')
        
        # Escape special characters and convert to one line
        content=$(cat "$f" | sed 's/"/\\"/g' | tr '\n' ' ')
        
        if [ "$FIRST_DOC" = true ]; then
            FIRST_DOC=false
        else
            DOCS_JSON="$DOCS_JSON,"
        fi
        
        echo "Processing: $filename (key: $docKey)"
        DOCS_JSON="$DOCS_JSON{\"@search.action\":\"upload\",\"id\":\"$docKey\",\"content\":\"$content\"}"
    fi
done

DOCS_JSON="$DOCS_JSON]}"

# Debug output
echo -e "\nPrepared JSON payload (first 500 chars):"
echo "${DOCS_JSON:0:500}..."

# Upload documents
echo -e "\nUploading to Azure AI Search..."
RESPONSE=$(curl -s -X POST \
    "$AZURE_SEARCH_ENDPOINT/indexes/$AZURE_SEARCH_INDEX/docs/index?api-version=2021-04-30-Preview" \
    -H "Content-Type: application/json" \
    -H "api-key: $AZURE_SEARCH_ADMIN_KEY" \
    -d "$DOCS_JSON")

# Check response - look for actual error conditions
if echo "$RESPONSE" | jq -e '.error' >/dev/null 2>&1; then
    echo "Error uploading documents:"
    echo "$RESPONSE" | jq '.'
    exit 1
else
    # Show success message with document statuses
    echo "Success! Documents uploaded to index: $AZURE_SEARCH_INDEX"
    echo -e "\nDocument statuses:"
    echo "$RESPONSE" | jq -r '.value[] | "- \(.key): \(.status) (status code: \(.statusCode))"'
fi

echo ""
echo "Ingestion completed."
