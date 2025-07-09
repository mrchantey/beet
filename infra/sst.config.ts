/// <reference path="./.sst/platform/config.d.ts" />

// TODO get these from Cargo.toml or beet.toml

/// the function name matching the cargo lambda deploy step,
/// by default binary name in kebab-case (underscores not allowed)
// let app_name = "beet-new-web";

const appName = "BeetServer";
const domainName = "beetrsx.dev";

export default $config({
  app(input) {
    return {
      name: appName,
      removal: input?.stage === "production" ? "retain" : "remove",
      // protect: ["production"].includes(input?.stage),
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

    let domainPrefix = $app.stage === "production" ? "" : `${$app.stage}.`;

    let gateway = new sst.aws.ApiGatewayV2(`${appName}Gateway`, {
      domain: {
        name: `${domainPrefix}${domainName}`,
        /*
Cloudflare DNS requires two environment variables:
- CLOUDFLARE_API_TOKEN = https://dash.cloudflare.com/profile/api-tokens > Create Token > edit zone DNS > Copy Token
- CLOUDFLARE_DEFAULT_ACCOUNT_ID = https://dash.cloudflare.com/login > right click 'My Account' hamburger > Copy Account ID
        */
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

    let func = new sst.aws.Function(`${appName}Lambda`, {
      // this name *must* match beet deploy --function-name ...
      name: `${appName}Lambda`,
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

    // else try func.arn
    let route = gateway.route("$default", func.arn);
  },
});
