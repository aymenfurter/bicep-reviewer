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
