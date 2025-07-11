+++
title= "Infrastructure"
+++

Beet comes with an [`sst`](https://sst.dev/) configuration for managing infrastructure.

## `beet infra`

This command is a lightweight wrapper of `npx sst`, with some added conventions.

**infra directory**

Rust is currently not well supported in sst, for example it refuses to deploy without building a binary, but doesnt allow feature flags etc. The infra directory has some hacks to get around this, providing a dummy `Cargo.toml` and `bootstrap` file. For this reason `beet infra` will cd into the infra directory for all sst commands.


**default stages**

Beet will default to the `dev` stage, using `beet infra --release` will use the `prod` stage instead.

## Tips

SST is a great tool, but does take some practice to build up an intuition for how IaC works.

### Dangling Resources

Its important to call `beet infra remove` before editing the `sst.config.ts` file to avoid resources not being cleaned up. Also I'd recommend not interrupting an sst command.

### State Mismatch

Running sst commands will keep a local record of the state of the application infrastructure. If these become out of sync, for example if a resource is manually created or removed, `sst deploy` will error. This can be fixed with `beet infra refresh`


## Custom Domains

sst can hook up custom domains, this guide will use cloudflare which has an excellent reputation as a domain registrar.

1. Purchase a domain at `https://domains.cloudflare.com/`
2. Configure the cloudflare environment variables:
	- `CLOUDFLARE_DEFAULT_ACCOUNT_ID`: https://dash.cloudflare.com/login > 'Bobs Account' hamburger > Copy Account ID
	- `CLOUDFLARE_API_TOKEN`: https://dash.cloudflare.com/profile/api-tokens > Create Token > edit zone DNS > specify zone (recommended)> Continue To Summary > Create Token > Copy
3. Update the `sst.config.ts`, see [this site's config](https://github.com/mrchantey/beet/blob/main/infra/sst.config.ts) for an example
4. Run `beet infra`



### AWS Credentials

AWS credentials are required for sst and cargo-lambda:

1. IAM -> User Groups -> Create Group: `sst-admin` -> Create
2. Permissions -> Add Permissions -> Create Inline Policy
3. Copy below -> next -> name: `sst-recommended-policy`
4. Users -> Create User -> `my-computer-name-sst-deploy` -> Next
5. User Groups -> Add `sst-admin` -> Next -> Create User
6. Users -> `my-computer-name-sst-deploy` -> Security Credentials -> Access Keys -> Create Access Key
7. In the cli `aws configure`, copy-paste Access Key ID & Secret

#### SST Recommended Policy

```json
{
	"Version": "2012-10-17",
	"Statement": [
		{
			"Sid": "ManageBootstrapStateBucket",
			"Effect": "Allow",
			"Action": [
				"s3:CreateBucket",
				"s3:PutBucketVersioning",
				"s3:PutBucketNotification",
				"s3:PutBucketPolicy",
				"s3:DeleteObject",
				"s3:GetObject",
				"s3:ListBucket",
				"s3:PutObject"
			],
			"Resource": [
				"arn:aws:s3:::sst-state-*"
			]
		},
		{
			"Sid": "ManageBootstrapAssetBucket",
			"Effect": "Allow",
			"Action": [
				"s3:CreateBucket",
				"s3:PutBucketVersioning",
				"s3:PutBucketNotification",
				"s3:PutBucketPolicy",
				"s3:DeleteObject",
				"s3:GetObject",
				"s3:ListBucket",
				"s3:PutObject"
			],
			"Resource": [
				"arn:aws:s3:::sst-asset-*"
			]
		},
		{
			"Sid": "ManageBootstrapECRRepo",
			"Effect": "Allow",
			"Action": [
				"ecr:CreateRepository",
				"ecr:DescribeRepositories"
			],
			"Resource": [
				"arn:aws:ecr:us-west-2:898915787211:repository/sst-asset"
			]
		},
		{
			"Sid": "ManageBootstrapSSMParameter",
			"Effect": "Allow",
			"Action": [
				"ssm:GetParameters",
				"ssm:PutParameter"
			],
			"Resource": [
				"arn:aws:ssm:us-west-2:898915787211:parameter/sst/passphrase/*",
				"arn:aws:ssm:us-west-2:898915787211:parameter/sst/bootstrap"
			]
		},
		{
			"Sid": "Deployments",
			"Effect": "Allow",
			"Action": [
				"*"
			],
			"Resource": [
				"*"
			]
		},
		{
			"Sid": "ManageSecrets",
			"Effect": "Allow",
			"Action": [
				"ssm:DeleteParameter",
				"ssm:GetParameter",
				"ssm:GetParameters",
				"ssm:GetParametersByPath",
				"ssm:PutParameter"
			],
			"Resource": [
				"arn:aws:ssm:us-west-2:898915787211:parameter/sst/*"
			]
		},
		{
			"Sid": "LiveLambdaSocketConnection",
			"Effect": "Allow",
			"Action": [
				"appsync:EventSubscribe",
				"appsync:EventPublish",
				"appsync:EventConnect"
			],
			"Resource": [
				"*"
			]
		}
	]
}
```