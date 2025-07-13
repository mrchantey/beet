import * as aws from "@pulumi/aws";
import * as cloudflare from "@pulumi/cloudflare";
import * as pulumi from "@pulumi/pulumi";

// Provider configuration
const awsProvider = new aws.Provider("aws", {
	region: "us-west-2",
});

const awsUsEast1Provider = new aws.Provider("us-east-1", {
	region: "us-east-1",
});

const cloudflareProvider = new cloudflare.Provider("cloudflare", {
	apiToken: process.env.CLOUDFLARE_API_TOKEN,
});

// Configuration
const stack = pulumi.getStack();

// App configuration
const appName = "beet-site";
const domainName = "beetstack.dev";
const prodStage = "prod";

// Add stack-based protection
const protectResources = stack === prodStage;

// The production stage has no prefix
const domainPrefix = stack === prodStage ? "" : `${stack}.`;
const fullDomainName = `${domainPrefix}${domainName}`;

// Consistent resource naming
const resourceName = (resourceType: string) =>
	`${appName}-${resourceType}-${stack}`;

// IAM role for the Lambda function
const lambdaRole = new aws.iam.Role(resourceName("lambda-role"), {
	assumeRolePolicy: JSON.stringify({
		Version: "2012-10-17",
		Statement: [
			{
				Action: "sts:AssumeRole",
				Effect: "Allow",
				Principal: {
					Service: "lambda.amazonaws.com",
				},
			},
		],
	}),
}, { provider: awsProvider });

// Attach basic execution role policy
const basicExecutionRolePolicy = new aws.iam.RolePolicyAttachment(
	resourceName("lambda-basic-execution"),
	{
		role: lambdaRole.name,
		policyArn: aws.iam.ManagedPolicy.AWSLambdaBasicExecutionRole,
	},
	{ provider: awsProvider },
);

// Custom policy for S3 and CloudWatch logs
const lambdaPolicy = new aws.iam.RolePolicy(resourceName("lambda-policy"), {
	role: lambdaRole.id,
	policy: JSON.stringify({
		Version: "2012-10-17",
		Statement: [
			{
				Effect: "Allow",
				Action: [
					"s3:*",
				],
				Resource: "*",
			},
			{
				Effect: "Allow",
				Action: [
					"logs:CreateLogGroup",
					"logs:CreateLogStream",
					"logs:PutLogEvents",
				],
				Resource: "*",
			},
		],
	}),
}, { provider: awsProvider });

// Lambda function
const lambdaFunction = new aws.lambda.Function(resourceName("lambda"), {
	name: resourceName("lambda"),
	role: lambdaRole.arn,
	runtime: "provided.al2023", // Rust runtime
	handler: "bootstrap",
	code: new pulumi.asset.AssetArchive({
		"bootstrap": new pulumi.asset.FileAsset("./bootstrap"),
	}),
	timeout: 180, // 3 minutes
	memorySize: 128, // 128 MB
}, {
	dependsOn: [basicExecutionRolePolicy, lambdaPolicy],
	provider: awsProvider,
	protect: protectResources,
});

// Lambda permission for API Gateway
new aws.lambda.Permission(resourceName("lambda-permission"), {
	action: "lambda:InvokeFunction",
	function: lambdaFunction.name,
	principal: "apigateway.amazonaws.com",
	sourceArn: pulumi.interpolate`arn:aws:execute-api:us-west-2:${
		aws.getCallerIdentity().then((id: any) => id.accountId)
	}:*/*/*`,
}, { provider: awsProvider });

// API Gateway V2 (HTTP API)
const apiGateway = new aws.apigatewayv2.Api(resourceName("gateway"), {
	name: resourceName("gateway"),
	protocolType: "HTTP",
	corsConfiguration: {
		allowCredentials: true,
		allowHeaders: ["*"],
		allowMethods: ["*"],
		allowOrigins: ["*"],
		exposeHeaders: ["*"],
		maxAge: 86400,
	},
}, {
	provider: awsProvider,
	protect: protectResources,
});

// Lambda integration
const lambdaIntegration = new aws.apigatewayv2.Integration(
	resourceName("lambda-integration"),
	{
		apiId: apiGateway.id,
		integrationType: "AWS_PROXY",
		integrationUri: lambdaFunction.invokeArn,
		integrationMethod: "POST",
		payloadFormatVersion: "2.0",
	},
	{ provider: awsProvider },
);

// Default route
const defaultRoute = new aws.apigatewayv2.Route(resourceName("default-route"), {
	apiId: apiGateway.id,
	routeKey: "$default",
	target: pulumi.interpolate`integrations/${lambdaIntegration.id}`,
}, { provider: awsProvider });

// Auto-deployment stage
const stage = new aws.apigatewayv2.Stage(resourceName("stage"), {
	apiId: apiGateway.id,
	name: "$default",
	autoDeploy: true,
}, { dependsOn: [defaultRoute], provider: awsProvider });

// Get the hosted zone for the domain
const hostedZone = cloudflare.getZoneOutput({
	name: domainName,
}, { provider: cloudflareProvider });

// ACM certificate for the domain
const certificate = new aws.acm.Certificate(resourceName("cert"), {
	domainName: fullDomainName,
	validationMethod: "DNS",
}, {
	provider: awsUsEast1Provider,
	protect: protectResources
}) // CloudFront requires certs in us-east-1

// DNS validation records in Cloudflare
const certValidationRecords = certificate.domainValidationOptions.apply((
	options: any,
) =>
	options.map((option: any, index: number) =>
		new cloudflare.Record(resourceName(`cert-validation-${index}`), {
			zoneId: hostedZone.id,
			name: option.resourceRecordName,
			value: option.resourceRecordValue,
			type: option.resourceRecordType,
			ttl: 60,
		}, { provider: cloudflareProvider })
	)
);

// Certificate validation
const certValidation = new aws.acm.CertificateValidation(
	resourceName("cert-validation"),
	{
		certificateArn: certificate.arn,
		validationRecordFqdns: pulumi.all(certValidationRecords).apply((
			records: any,
		) => records.map((record: any) => record.hostname)),
	},
	{ provider: awsUsEast1Provider },
);

// Custom domain for API Gateway
const domainName_resource = new aws.apigatewayv2.DomainName(
	resourceName("domain"),
	{
		domainName: fullDomainName,
		domainNameConfiguration: {
			certificateArn: certValidation.certificateArn,
			endpointType: "REGIONAL",
			securityPolicy: "TLS_1_2",
		},
	},
	{
		provider: awsProvider,
		protect: protectResources,
	},
);

// API mapping
const apiMapping = new aws.apigatewayv2.ApiMapping(
	resourceName("api-mapping"),
	{
		apiId: apiGateway.id,
		domainName: domainName_resource.id,
		stage: stage.id,
	},
	{ provider: awsProvider },
);

// DNS record pointing to the API Gateway domain
new cloudflare.Record(resourceName("dns"), {
	zoneId: hostedZone.id,
	name: domainPrefix ? domainPrefix.slice(0, -1) : "@", // Remove trailing dot for subdomain, use @ for root
	value: domainName_resource.domainNameConfiguration.targetDomainName,
	type: "CNAME",
	ttl: 300,
}, {
	dependsOn: [apiMapping],
	provider: cloudflareProvider,
	protect: protectResources,
});

// Function URL (equivalent to SST's url: true)
const functionUrl = new aws.lambda.FunctionUrl(resourceName("function-url"), {
	functionName: lambdaFunction.name,
	authorizationType: "NONE",
	cors: {
		allowCredentials: true,
		allowHeaders: ["*"],
		allowMethods: ["*"],
		allowOrigins: ["*"],
		exposeHeaders: ["*"],
		maxAge: 86400,
	},
}, { provider: awsProvider });

// Exports
export const apiGatewayUrl = apiGateway.apiEndpoint;
export const customDomainUrl = pulumi.interpolate`https://${fullDomainName}`;
export const functionUrlOutput = functionUrl.functionUrl;
export const lambdaFunctionName = lambdaFunction.name;
