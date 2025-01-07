# Bicep Best Practices

## Training Resources
For step-by-step guidance on Bicep best practices, refer to [Structure your Bicep code for collaboration](https://learn.microsoft.com/en-us/azure/azure-resource-manager/bicep/file-structure).

## Parameters
- Use descriptive names that are consistent and easy to understand
- Reserve parameters for settings that vary between deployments
- Set safe default values to prevent unexpected costs
- Use `@allowed` decorator sparingly to maintain flexibility
- Include helpful parameter descriptions
- Place parameter declarations at the top of the file
- Specify minimum and maximum character lengths for naming parameters

For more details, see [Parameters in Bicep](https://learn.microsoft.com/en-us/azure/azure-resource-manager/bicep/parameters).

## Variables
- Data types are inferred automatically
- Can incorporate Bicep functions
- Reference using variable name
- Use variables to simplify complex expressions

For more details, see [Variables in Bicep](https://learn.microsoft.com/en-us/azure/azure-resource-manager/bicep/variables).

## Naming Conventions
- Use lowerCamelCase (e.g., `myVariableName`, `myResource`)
- Utilize `uniqueString()` for unique resource names
- Create meaningful resource names using template expressions:
```bicep
param shortAppName string = 'toy'
param shortEnvironmentName string = 'prod'
param appServiceAppName string = '${shortAppName}-${shortEnvironmentName}-${uniqueString(resourceGroup().id)}'
```
- Avoid using 'name' in symbolic names
- Don't use suffixes to distinguish variables and parameters

## Resource Definitions
- Use variables for complex expressions
- Reference resource properties directly for outputs
- Use recent API versions
- Prefer symbolic names over `reference()` and `resourceId()`
- Use implicit dependencies over explicit `dependsOn`
- Use `existing` keyword for external resources

## Child Resources
- Minimize nesting depth
- Use parent property instead of constructing resource names
- Maintain clear resource relationships

## Outputs
- Never expose sensitive data in outputs
- Use `existing` keyword to look up properties
- Keep outputs focused on necessary values

## Tenant Scope Operations
- Deploy organization-wide policies and role assignments to root management group
- Note that direct tenant-scope operations are limited


