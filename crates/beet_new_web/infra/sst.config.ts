/// <reference path="./.sst/platform/config.d.ts" />

/// the function name matching the cargo lambda deploy step,
/// by default binary name in kebab-case (underscores not allowed)
let app_name = "beet-new-web";

export default $config({
  app(input) {
    return {
      name: app_name,
      removal: input?.stage === "production" ? "retain" : "remove",
      home: "aws",
      providers: {
        aws: {
          region: "us-west-2",
        },
      },
    };
  },
  async run() {
    console.log(
      `🌱 Deploying Lambda function: ${app_name} - stage: ${$app.stage}`,
    );

    let func = new sst.aws.Function(`${app_name}-lambda`, {
      name: `${app_name}-lambda`,
      runtime: "rust",
      // we'll upload the real handler with cargo-lambda
      handler: "./dummy",
      url: true,
      timeout: "3 minutes",
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
  },
});
