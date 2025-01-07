@description('Name of the App Service plan')
param appServicePlanName string

@description('Name of the Web App')
param webAppName string

@description('Supported SKU for the App Service plan')
@allowed([
  'F1'
  'B1'
])
param skuName string = 'F1'

resource appServicePlan 'Microsoft.Web/serverfarms@2022-03-01' = {
  name: appServicePlanName
  location: resourceGroup().location
  sku: {
    name: skuName
    capacity: 1
  }
}

resource webApp 'Microsoft.Web/sites@2022-03-01' = {
  name: webAppName
  location: resourceGroup().location
  properties: {
    serverFarmId: appServicePlan.id
  }
}

output webAppHostname string = webApp.defaultHostName
