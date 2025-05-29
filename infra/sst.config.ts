/// <reference path="./.sst/platform/config.d.ts" />

// TODO get these from Cargo.toml or beet.toml
const appName = "BeetServer";
const domainName = "beetrs.dev";
const lambdaHandler = "./crates/beet_site/";

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
      // doesnt work but this is the one we want, we're not using
      // sst for lambda deployment
      // runtime: "provided.al2023",
      // what is this used for?
      handler: lambdaHandler,
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
