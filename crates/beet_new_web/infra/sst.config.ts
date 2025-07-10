/// <reference path="./.sst/platform/config.d.ts" />

/// the function name matching the cargo lambda deploy step,
/// by default binary name in kebab-case (underscores not allowed)
let appName = "beet-new-web";

export default $config({
  app(input) {
    return {
      name: appName,
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
    // consistent resource naming
    let resourceName = (resourceType: string) =>
      `${appName}-${resourceType}-${$app.stage}`;

    let func = new sst.aws.Function(resourceName("lambda"), {
      name: resourceName("lambda"),
      runtime: "rust",
      // point to this dummy Cargo.toml
      handler: "",
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
