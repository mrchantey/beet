/// <reference path="./.sst/platform/config.d.ts" />

/// the function name matching the cargo lambda deploy step,
/// by default binary name in kebab-case (underscores not allowed)
// let app_name = "beet-new-web";

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
    console.log(`🌱 Deploying Infrastructure - stage: ${$app.stage}`);

    let domainPrefix = $app.stage === prodStage ? "" : `${$app.stage}.`;

    // 1. create the api gateway
    let gateway = new sst.aws.ApiGatewayV2(`${appName}-${$app.stage}-gateway`, {
      domain: {
        name: `${domainPrefix}${domainName}`,
        dns: sst.cloudflare.dns(),
      },
      cors: true,
      transform: {
        stage: (args) => {
          args.name = "$default";
          args.autoDeploy = true;
        },
      },
    });
    // 2. create the lambda function
    let func = new sst.aws.Function(`${appName}-${$app.stage}-lambda`, {
      // this name *must* match beet deploy --function-name ...
      name: `${appName}-${$app.stage}-lambda`,
      // the rust runtime is not ready, we deploy ourselves
      runtime: "rust",
      // we'll upload the real handler with cargo-lambda
      handler: "./dummy",
      url: true,
      timeout: "3 minutes",
      // memory: "1024 MB"
      permissions: [
        {
          actions: [
            "s3:*",
          ],
          resources: ["*"],
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

    // 3. point the gateway's default route to the function
    let route = gateway.route("$default", func.arn);
  },
});
