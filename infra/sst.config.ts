/// <reference path="./.sst/platform/config.d.ts" />



export default $config({
  app(input) {
    return {
      name: "BeetSite",
      removal: input?.stage === "production" ? "retain" : "remove",
      // protect: ["production"].includes(input?.stage),
      home: "aws",
      providers: {
        aws: {
          region: "us-west-2", 
        }
      }
    };
  },
  async run() {

    new sst.aws.Function("BeetSiteLambda", {
      // this name *must* match beet deploy --function-name ...
      name: "BeetSiteLambda",
      // the rust runtime is not ready, we deploy ourselves
      runtime: 'rust',
      // doesnt work but this is the one we want, we're not using
      // sst for lambda deployment
      // runtime: "provided.al2023", 
      // what is this used for?
      handler: "./crates/beet_site/",
      url: true,
      timeout: "3 minutes",
      // memory: "1024 MB"
      permissions: [
        {
          actions: [
        "s3:*"
          ],
          resources: ["*"]
        },
        {
          actions: [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
          ],
          resources: ["*"]
        }
      ],
    });
  },
});
