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
  run() {
    const domainPrefix = $app.stage === prodStage ? "" : `${$app.stage}.`;

    // consistent resource naming
    const resourceName = (resourceType: string) =>
      `${appName}-${resourceType}-${$app.stage}`;

    // 1. create the s3 bucket for serving static html
    const _bucket = new sst.aws.Bucket(resourceName("bucket"), {
      name: resourceName("bucket"),
      access: "public",
      transform: {
        bucket: (args: any) => {
          args.bucket = resourceName("bucket");
        },
      },
    });

    // 2. create the api gateway
    const gateway = new sst.aws.ApiGatewayV2(resourceName("gateway"), {
      domain: {
        name: `${domainPrefix}${domainName}`,
        dns: sst.cloudflare.dns(),
      },
      cors: true,
      transform: {
        stage: (args: any) => {
          args.name = "$default";
          args.autoDeploy = true;
        },
      },
    });
    // 3. create the lambda function
    const func = new sst.aws.Function(resourceName("lambda"), {
      // this name *must* match RunInfra::lambda_func_name
      name: resourceName("lambda"),
      // the rust runtime is not ready, we deploy ourselves
      runtime: "rust",
      // point to this dummy Cargo.toml
      handler: "",
      url: true,
      timeout: "3 minutes",
      // memory: "1024 MB"
      permissions: [
        {
          actions: [
            "s3:*",
          ],
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
