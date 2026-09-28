//! The IAM policy a repo's deployer needs, LOWERED from the provider types the
//! stacks it deploys declare.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// The deploy-side counterpart of [`IamPolicy`]: that lowers a stack's
/// [`AccessGrant`]s into what the RUNNING process may do, this lowers the
/// provider types a stack RENDERS into what the deployer applying it may do.
/// One policy per app: every stage of it by name, and the services the
/// launch's own stage renders.
///
/// The input is the rendered [`terra::Config`] and nothing else, so the policy
/// follows the declarations: a block added to a stack widens it on the next
/// mint, and a type this lowering does not know FAILS naming it, the same
/// loud-on-unknown rule grant lowering has and for the same reason (a silently
/// dropped service is a deploy that works until the apply that touches it).
///
/// ## What it grants
///
/// - `sts:GetCallerIdentity`, which every launch calls to derive its state
///   bucket, and any global service the stacks use.
/// - the state bucket, read-only at the bucket level and read/write on the keys
///   named for this app alone (`<app>--*`, which is every stage's state object
///   and the lock beside each), so one app's deployer cannot touch another's
///   state.
/// - `s3:*` on this app's own bucket namespace (`<app>--*`), which is every
///   bucket a stack of it composes, in any stage.
/// - `iam:*` on this app's own roles, users, policies and instance profiles
///   (`<app>--*`), when a stack renders any; a stack with no IAM resource gets
///   no IAM statement at all.
/// - `ssm:*` under this app's own parameter prefix (`/<app>/`), the stack's
///   secret store, plus AWS's public `/aws/service/` tree (an AMI id) and the
///   one parameter call that takes no resource, `ssm:DescribeParameters`.
/// - `<service>:*` on `*` for every other service, conditioned on the regions
///   the stacks declare.
///
/// The last line is where this stops being least privilege: an action-level
/// policy per service would be a list nobody can verify without applying every
/// path of every stack, and a provider refresh reads far more than a deploy
/// writes. What the scoping does buy is that a leaked deployer key is one app's
/// blast radius in one region, with no IAM outside that app's names, no access
/// to another app's buckets or state, and nothing at the account level.
///
/// Every name pattern is per APP and not per stage, so one mint covers every
/// stage of it. That is not a scoping the pair has to give up: a credential
/// document is per repo, so the same key deploys `dev` and `prod`, and a policy
/// split by stage would draw a boundary the key does not have.
///
/// The SERVICES are still one stage's, because one launch renders one stage and
/// a stage may declare more (a prod site has a certificate and a custom domain
/// where its dev stage has neither). So a mint runs under the stage that
/// actually deploys, `--stage=prod` where that is not the default, which the
/// mint verb warns about when it is not (`DeployerMint`, native and `deploy`,
/// so not linkable from here).
///
/// ## What it cannot know
///
/// A bucket outside the app's namespace (a `<StoreUriBlock>` naming another
/// app's store) and a resource the config no longer declares but the state
/// still holds (a destroy in progress) both need a statement by hand.
#[derive(Debug, Clone)]
pub struct DeployerPolicy {
	/// The app every name is scoped to, ie `beet-site`.
	app: SmolStr,
	/// The state bucket this launch resolved, ie `beet-state-<account id>`.
	state_bucket: SmolStr,
	/// The regions the stacks declare, which every regional service is
	/// conditioned on.
	regions: BTreeSet<SmolStr>,
	/// The IAM services the lowered types named.
	services: BTreeSet<&'static str>,
}

impl DeployerPolicy {
	/// Which IAM service a provider type's api calls belong to, by longest
	/// matching prefix, so a service's whole family rides one entry and an
	/// exception (`aws_cloudwatch_log_group` is `logs`, not `cloudwatch`)
	/// rides a longer one. Order is not load bearing.
	///
	/// A type beet renders through `add_untyped_resource` is covered by its
	/// family's prefix, which is why these are prefixes rather than the
	/// closed list of the bindings.
	const SERVICES: &'static [(&'static str, &'static str)] = &[
		("aws_acm_", "acm"),
		("aws_api_gateway_", "apigateway"),
		("aws_apigatewayv2_", "apigateway"),
		("aws_appautoscaling_", "application-autoscaling"),
		("aws_availability_zones", "ec2"),
		("aws_caller_identity", "sts"),
		("aws_cloudwatch_log_", "logs"),
		("aws_cloudwatch_", "cloudwatch"),
		("aws_db_", "rds"),
		("aws_dynamodb_", "dynamodb"),
		("aws_ebs_", "ec2"),
		("aws_ecr_", "ecr"),
		("aws_ecs_", "ecs"),
		("aws_eip", "ec2"),
		("aws_iam_", "iam"),
		("aws_instance", "ec2"),
		("aws_internet_gateway", "ec2"),
		("aws_key_pair", "ec2"),
		("aws_lambda_", "lambda"),
		("aws_lb", "elasticloadbalancing"),
		("aws_lightsail_", "lightsail"),
		("aws_nat_gateway", "ec2"),
		("aws_network_interface", "ec2"),
		("aws_rds_", "rds"),
		("aws_region", "ec2"),
		("aws_route53_", "route53"),
		("aws_route_table", "ec2"),
		("aws_route", "ec2"),
		("aws_s3_", "s3"),
		("aws_scheduler_", "scheduler"),
		("aws_security_group", "ec2"),
		("aws_ses", "ses"),
		("aws_sns_", "sns"),
		("aws_sqs_", "sqs"),
		("aws_ssm_", "ssm"),
		("aws_subnet", "ec2"),
		("aws_volume_attachment", "ec2"),
		("aws_vpc", "ec2"),
	];

	/// The services whose statements are scoped by NAME rather than by the
	/// regional wildcard: each has its own statement below, and including it
	/// in the wildcard one would undo the scoping.
	const NAME_SCOPED: &'static [&'static str] = &["iam", "s3", "ssm"];

	/// Services whose endpoint is global, so `aws:RequestedRegion` is
	/// `us-east-1` however the stack is configured and the regional condition
	/// would deny every call.
	const GLOBAL: &'static [&'static str] = &["route53", "sts"];

	/// Provider prefixes no IAM policy grants, for one of two reasons: the
	/// provider answers to its own credential (Cloudflare's account token), or
	/// it makes no remote call at all (terraform's own builtins, the local
	/// providers). Absence here is not a silent pass: a prefix this does not
	/// name and [`SERVICES`](Self::SERVICES) does not place FAILS the mint.
	const NO_GRANT: &'static [&'static str] = &[
		"cloudflare_",
		"local_",
		"null_",
		"random_",
		"terraform_",
		"time_",
		"tls_",
	];

	/// A policy for `app`, whose stacks keep their state in `state_bucket`.
	pub fn new(
		app: impl Into<SmolStr>,
		state_bucket: impl Into<SmolStr>,
	) -> Self {
		Self {
			app: app.into(),
			state_bucket: state_bucket.into(),
			regions: BTreeSet::new(),
			services: BTreeSet::new(),
		}
	}

	/// The managed policy this lowering writes, ie `beet-site--deploy`.
	/// Deliberately outside the `<app>--<stage>--<label>` convention every
	/// resource name follows, so no stack's own IAM statement can reach the
	/// policy that grants it.
	pub fn policy_name(&self) -> String { format!("{}--deploy", self.app) }

	/// Lower one of the app's stacks: the services its declared types name and
	/// the region its regional statement is conditioned on. Fails naming any
	/// type this lowering cannot place, and naming a stack whose app is not
	/// this policy's.
	pub fn lower(
		mut self,
		stack: &ResolvedStack,
		config: &terra::Config,
	) -> Result<Self> {
		if stack.app_name() != &self.app {
			bevybail!(
				"stack `{}--{}` is not an app `{}` stack: one policy covers one \
				app's stages",
				stack.app_name(),
				stack.stage(),
				self.app
			);
		}
		for declared in config.declared_types() {
			if let Some(service) = Self::service(declared)? {
				self.services.insert(service);
			}
		}
		self.regions.insert(stack.aws_region()?.clone());
		self.xok()
	}

	/// The IAM service `declared` belongs to, `None` for a provider no IAM
	/// policy grants ([`NO_GRANT`](Self::NO_GRANT)), an error for a type with
	/// no entry at all.
	fn service(declared: &str) -> Result<Option<&'static str>> {
		if Self::NO_GRANT
			.iter()
			.any(|prefix| declared.starts_with(prefix))
		{
			return None.xok();
		}
		Self::SERVICES
			.iter()
			.filter(|(prefix, _)| declared.starts_with(prefix))
			.max_by_key(|(prefix, _)| prefix.len())
			.map(|(_, service)| Some(*service))
			.ok_or_else(|| {
				bevyhow!(
					"no deployer permission is declared for `{declared}`: add \
					its service prefix to `DeployerPolicy::SERVICES`"
				)
			})
	}

	/// The policy document, ready for `iam create-policy`.
	pub fn to_json(&self) -> Value {
		let mut statements = vec![json!({
			"Sid": "Global",
			"Effect": "Allow",
			"Action": self.global_actions(),
			"Resource": "*",
		})];
		if let Some(regional) = self.regional_actions() {
			statements.push(json!({
				"Sid": "Regional",
				"Effect": "Allow",
				"Action": regional,
				"Resource": "*",
				// every regional service in one condition: a deployer that
				// cannot call outside its stacks' regions cannot quietly run a
				// workload in one nobody looks at
				"Condition": {
					"StringEquals": {
						"aws:RequestedRegion": self.regions.iter().collect::<Vec<_>>(),
					}
				},
			}));
		}
		// the state bucket itself is shared by every app in the account, so
		// only its keys are scoped: the bucket-level reads are what
		// `ensure_exists` and the region discovery call on every verb
		statements.push(json!({
			"Sid": "StateBucket",
			"Effect": "Allow",
			"Action": [
				"s3:ListBucket",
				"s3:GetBucketLocation",
				"s3:GetBucketVersioning",
			],
			"Resource": format!("arn:aws:s3:::{}", self.state_bucket),
		}));
		statements.push(json!({
			"Sid": "State",
			"Effect": "Allow",
			"Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
			// every key named for this app: each stage's state object
			// (`<app>--<stage>--tofu-tfstate`) and the `.tflock` beside it
			"Resource": format!(
				"arn:aws:s3:::{}/{}--*",
				self.state_bucket, self.app
			),
		}));
		if self.services.contains("s3") {
			statements.push(json!({
				"Sid": "Buckets",
				"Effect": "Allow",
				"Action": "s3:*",
				"Resource": [
					format!("arn:aws:s3:::{}--*", self.app),
					format!("arn:aws:s3:::{}--*/*", self.app),
				],
			}));
		}
		if self.services.contains("iam") {
			statements.push(json!({
				"Sid": "Iam",
				"Effect": "Allow",
				"Action": "iam:*",
				"Resource": self.iam_resources(),
			}));
			statements.push(json!({
				"Sid": "IamManagedRead",
				"Effect": "Allow",
				// an attachment of an aws-managed policy reads it first
				"Action": [
					"iam:GetPolicy",
					"iam:GetPolicyVersion",
					"iam:ListPolicyVersions",
				],
				"Resource": "arn:aws:iam::aws:policy/*",
			}));
		}
		if self.services.contains("ssm") {
			statements.push(json!({
				"Sid": "Ssm",
				"Effect": "Allow",
				"Action": "ssm:*",
				"Resource": format!("arn:aws:ssm:*:*:parameter/{}/*", self.app),
			}));
			// AWS's own public parameter tree, which a compute block reads its
			// AMI id out of: public, and outside every app's prefix
			statements.push(json!({
				"Sid": "SsmPublic",
				"Effect": "Allow",
				"Action": ["ssm:GetParameter", "ssm:GetParameters"],
				"Resource": "arn:aws:ssm:*:*:parameter/aws/service/*",
			}));
			// `DescribeParameters` supports no resource-level permission at
			// all, so `*` is the only form it takes; it answers names and
			// descriptions, never a value, and every provider refresh of a
			// parameter calls it
			statements.push(json!({
				"Sid": "SsmDescribe",
				"Effect": "Allow",
				"Action": "ssm:DescribeParameters",
				"Resource": "*",
			}));
		}
		json!({ "Version": "2012-10-17", "Statement": statements })
	}

	/// The unconditioned statement's actions: the identity call every launch
	/// makes, plus every global service the stacks use.
	fn global_actions(&self) -> Vec<String> {
		["sts:GetCallerIdentity".to_string()]
			.into_iter()
			.chain(
				self.services
					.iter()
					.filter(|service| {
						**service != "sts" && Self::GLOBAL.contains(service)
					})
					.map(|service| format!("{service}:*")),
			)
			.collect()
	}

	/// The region-conditioned statement's actions, `None` when every service
	/// the stacks use has a statement of its own.
	fn regional_actions(&self) -> Option<Vec<String>> {
		let actions = self
			.services
			.iter()
			.filter(|service| {
				!Self::NAME_SCOPED.contains(service)
					&& !Self::GLOBAL.contains(service)
			})
			.map(|service| format!("{service}:*"))
			.collect::<Vec<_>>();
		(!actions.is_empty()).then_some(actions)
	}

	/// One IAM arn pattern per resource category, each under the app's own
	/// name, so a deployer can create the roles its stacks declare and reach
	/// nothing else, its own user and policy included (both are named for the
	/// repo and the policy, never `<app>--`).
	fn iam_resources(&self) -> Vec<String> {
		["role", "user", "policy", "instance-profile"]
			.into_iter()
			.map(|kind| format!("arn:aws:iam::*:{kind}/{}--*", self.app))
			.collect()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A type's service is its longest matching prefix, an unknown one is a
	/// loud error, and a Cloudflare resource lowers to nothing.
	#[beet_core::test]
	fn places_every_type() {
		let service = |declared| DeployerPolicy::service(declared).unwrap();
		service("aws_s3_bucket_versioning").xpect_eq(Some("s3"));
		// the longer prefix wins: a log group is `logs`, an alarm `cloudwatch`
		service("aws_cloudwatch_log_group").xpect_eq(Some("logs"));
		service("aws_cloudwatch_metric_alarm").xpect_eq(Some("cloudwatch"));
		service("aws_route_table_association").xpect_eq(Some("ec2"));
		service("aws_route53_record").xpect_eq(Some("route53"));
		service("aws_sesv2_email_identity").xpect_eq(Some("ses"));
		service("aws_caller_identity").xpect_eq(Some("sts"));
		service("cloudflare_r2_bucket").xpect_eq(None);
		// terraform's own builtin: no remote call, so no grant
		service("terraform_data").xpect_eq(None);
		DeployerPolicy::service("aws_kinesis_stream")
			.unwrap_err()
			.to_string()
			.xpect_contains("aws_kinesis_stream");
	}

	/// The document of a bucket-only app: its own namespace, its own state
	/// key, no IAM statement and no regional wildcard.
	#[beet_core::test]
	fn lowers_a_bucket_stack() {
		let (scope, _dir) = RenderScope::test_render_stack(
			(
				Stack::new("my-egress").with_stage("prod"),
				AwsRegion::new("us-west-2"),
			),
			|parent| {
				parent.spawn(
					S3BucketBlock::new("store").with_deploy_versioned(false),
				);
			},
		);
		let (stack, _deployment, config) = scope.finish().unwrap();
		let policy = DeployerPolicy::new("my-egress", "beet-state-1234")
			.lower(&stack, &config)
			.unwrap();
		policy.policy_name().xpect_eq("my-egress--deploy");
		let document = policy.to_json().to_string();
		document
			.as_str()
			.xpect_contains("arn:aws:s3:::my-egress--*/*")
			// every stage of the app, so the mint does not depend on `--stage`
			.xpect_contains("arn:aws:s3:::beet-state-1234/my-egress--*")
			.xpect_contains("sts:GetCallerIdentity")
			// a bucket needs no other service, so there is nothing to condition
			.xnot()
			.xpect_contains("Regional")
			.xnot()
			.xpect_contains("iam:");
	}

	/// Two stages of one app share one policy and one state key pattern; a
	/// stack of another app is refused.
	#[beet_core::test]
	fn lowers_every_stage_of_one_app() {
		let render = |stage: &str| {
			let (scope, dir) = RenderScope::test_render_stack(
				(
					Stack::new("beet-site").with_stage(stage),
					AwsRegion::new("us-west-2"),
				),
				|parent| {
					parent.spawn(S3BucketBlock::new("repo"));
				},
			);
			(scope.finish().unwrap(), dir)
		};
		let ((prod, _, config), _prod_dir) = render("prod");
		let ((shared, _, shared_config), _shared_dir) = render("shared");
		let document = DeployerPolicy::new("beet-site", "beet-state-1234")
			.lower(&prod, &config)
			.unwrap()
			.lower(&shared, &shared_config)
			.unwrap()
			.to_json()
			.to_string();
		// one pattern covers both stages, so lowering the second adds nothing
		document
			.as_str()
			.xpect_contains("beet-state-1234/beet-site--*")
			.xnot()
			.xpect_contains("beet-site--prod--tofu-tfstate");
		DeployerPolicy::new("beet-site", "beet-state-1234")
			.lower(&Stack::new("other").resolve(&default()), &config)
			.unwrap_err()
			.to_string()
			.xpect_contains("one app's stages");
	}
}
