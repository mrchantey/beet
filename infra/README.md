# Beet Infrastructure with Pulumi

This directory contains the infrastructure-as-code for the Beet project using Pulumi instead of SST.

## Migration from SST

The previous SST configuration has been replaced with equivalent Pulumi TypeScript code that provides:

- AWS Lambda function with Rust runtime support
- API Gateway V2 (HTTP API) with CORS enabled
- Custom domain with SSL certificate (using ACM)
- DNS management via Cloudflare
- Function URL for direct Lambda access
- Proper IAM roles and permissions

## Prerequisites

1. Install Pulumi CLI: https://www.pulumi.com/docs/get-started/install/
2. Install Node.js dependencies:
   ```bash
   npm install
   ```
3. Configure AWS credentials (via AWS CLI, environment variables, or IAM roles)
4. Set Cloudflare API token as environment variable:
   ```bash
   export CLOUDFLARE_API_TOKEN=your_token_here
   ```

## Usage

### Initial Setup

1. Initialize Pulumi state (if not already done):
   ```bash
   pulumi login
   ```

2. Create a new stack (equivalent to SST stages):
   ```bash
   pulumi stack init dev
   # or
   pulumi stack init prod
   ```

3. Set required configuration (optional - providers are configured in code):
   ```bash
   # Optional: Override default region
   pulumi config set aws:region us-west-2
   
   # Or set via environment variable
   export AWS_REGION=us-west-2
   export CLOUDFLARE_API_TOKEN=your_token_here
   ```

### Deployment

Deploy the infrastructure:
```bash
pulumi up
```

### Key Differences from SST

1. **Stack Management**: Use `pulumi stack` commands instead of SST stages
2. **Configuration**: Providers configured in code with environment variables for secrets
3. **State Management**: Pulumi manages state in the Pulumi Cloud or your chosen backend
4. **Resource Naming**: Resources are named using the same pattern: `${appName}-${resourceType}-${stack}`
5. **Environment Variables**: Use `CLOUDFLARE_API_TOKEN` for Cloudflare authentication

### Lambda Deployment

The Lambda function expects a `bootstrap` binary (Rust executable). Make sure to:

1. Build your Rust Lambda function
2. Copy the binary to `infra/bootstrap` 
3. Run `pulumi up` to deploy the updated function

### Stack Outputs

The following outputs are available after deployment:
- `apiGatewayUrl`: The API Gateway endpoint URL
- `customDomainUrl`: The custom domain URL (e.g., https://dev.beetstack.dev)
- `functionUrlOutput`: Direct Lambda function URL
- `lambdaFunctionName`: The name of the deployed Lambda function

### Cleanup

To destroy the infrastructure:
```bash
pulumi destroy
```

## Configuration

The infrastructure is configured with the following defaults:
- **Domain**: `beetstack.dev`
- **Region**: `us-west-2`
- **Lambda Runtime**: `provided.al2023` (for Rust)
- **Lambda Timeout**: 3 minutes
- **Memory**: 128MB

These can be customized by modifying `index.ts` or using Pulumi configuration.
