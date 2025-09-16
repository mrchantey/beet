// deno-lint-ignore-file no-explicit-any
/// <reference path="./.sst/platform/config.d.ts" />

// the function name matching the cargo lambda deploy step,
// by default binary name in kebab-case (underscores not allowed)
// TODO get these from Cargo.toml/beet.toml
const appName = "beet-site";
const domainName = "beetstack.dev";
// the production stage has no prefix and extra protections
// against removal
const prodStage = "prod";

export default $config({
	app(input) {
		return {
			name: appName,
			removal: input?.stage === prodStage ? "retain" : "remove",
			// protect: [prodStage].includes(input?.stage),
			home: "aws",
			providers: {
				aws: {
					region: "us-west-2",
				},
			},
		};
	},
	async run() {
		// consistent resource naming
		const resourceName = (descriptor: string) =>
			`${appName}--${$app.stage}--${descriptor}`;

		const _assets_bucket = new sst.aws.Bucket(resourceName("assets"), {
			access: "public",
			versioning: true,
			transform: {
				bucket: (args: any) => {
					args.bucket = resourceName("assets");
				},
			},
		});
		const _html_bucket = new sst.aws.Bucket(resourceName("html"), {
			access: "public",
			transform: {
				bucket: (args) => {
					args.bucket = resourceName("html");
				},
			},
		});

		// 2. create the api gateway
		const domainPrefix = $app.stage === prodStage ? "" : `${$app.stage}.`;
		const gateway = new sst.aws.ApiGatewayV2(resourceName("gateway"), {
			domain: {
				name: `${domainPrefix}${domainName}`,
				dns: sst.cloudflare.dns(),
			},
			cors: true,
			transform: {
				api: (args) => {
					args.name = resourceName("gateway");
				},
				stage: (args) => {
					args.name = "$default";
					args.autoDeploy = true;
				},
			},
		});
		// 3. create the lambda function
		const func = new sst.aws.Function(resourceName("router"), {
			// this name *must* match RunInfra::lambda_func_name
			name: resourceName("router"),
			// the rust runtime is not ready, we deploy ourselves
			runtime: "rust",
			// point to this dummy Cargo.toml
			handler: "",
			url: true,
			timeout: "3 minutes",
			// memory: "1024 MB"
			permissions: [
				{
					actions: ["s3:*"],
					resources: [
						"*",
						// bucket.arn,
						// `${bucket.arn}/*`,
					],
				},
				{
					actions: [
						"logs:CreateLogGroup",
						"logs:CreateLogStream",
						"logs:PutLogEvents",
					],
					resources: ["*"],
				},
			],
		});

		// 4. point the gateway's default route to the function
		const _route = gateway.route("$default", func.arn);
	},
});
