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
/// The SERVICES are one stage's, and that is the design rather than a
/// limitation to route around. A launch renders one stage, a stage may declare
/// more than another (a prod site has a certificate and a custom domain where
/// its dev stage has neither), and a policy should describe the stage that
/// actually deploys. So a mint runs under that stage, `--stage=prod` where it
/// is not the default, and `DeployerMint` WARNS when a stack took a non-prod
/// launch stage. **That warning is not a bug to fix**: prod is a superset of
/// dev for every stack in this account today, the failure when it is not is a
/// loud `AccessDenied` on the next plan, and one re-mint fixes it.
///
/// A mint that re-launched itself per stage and unioned the services was
/// considered and rejected: a self-relaunch is a new failure surface bought
/// for a problem that has happened once and was caught by the warning that
/// already exists. Revisit only if a third stage appears, or if a stage ever
/// declares something prod does not, which is the one case this cannot cover.
///
/// ## What it cannot know, and what that looks like
///
/// Two cases need a statement no lowering can derive. Neither happens in this
/// account today, so the seam is added when a case appears rather than invented
/// for a hypothetical; what is recorded here is the SYMPTOM, because an
/// `AccessDenied` is what a reader actually meets.
///
/// - **A bucket outside the app's namespace**, ie a `<StoreUriBlock>` naming
///   another app's store. The deploy fails with `AccessDenied` on an
///   `s3:` call against a bucket whose name does not start `<app>--`, and no
///   re-mint helps, because the policy follows the declarations and the
///   declaration is in another app.
/// - **A resource the config no longer declares but the state still holds**, ie
///   a block removed before its resource was destroyed, or a destroy
///   interrupted partway. The policy is lowered from what the config declares,
///   so a destroy of something undeclared is denied. Put the declaration back,
///   destroy, then remove it.
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
	/// The stages the lowered stacks declare, which the irreversible deny is
	/// scoped to alongside `prod`. Collected rather than assumed: an app whose
	/// only stage is `shared` gets its buckets protected too, where a
	/// hard-coded `prod` named nothing that exists.
	stages: BTreeSet<SmolStr>,
	/// Whether every principal the app's stacks render already carries its
	/// boundary. Until it does, the IAM statement is unconditioned, which is
	/// the one-time state a migration passes through.
	boundary: bool,
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
			stages: BTreeSet::new(),
			boundary: false,
		}
	}

	/// Condition every IAM write on the app's [`RuntimeBoundary`], which is
	/// only safe once every principal already carries it: the `StringNotLike`
	/// deny on `iam:PermissionsBoundary` is TRUE for a principal that has
	/// none, so an apply that would attach the first one is denied. The mint
	/// decides by asking the account, so the order is enforced rather than
	/// remembered.
	pub fn with_boundary(mut self) -> Self {
		self.boundary = true;
		self
	}

	/// The arn pattern every one of this app's boundaries matches, one per
	/// stage ([`RuntimeBoundary::policy_name`]). A `StringLike` pattern rather
	/// than one arn, because a policy covers every stage of its app while a
	/// boundary's CONTENT is one stage's.
	fn boundary_pattern(&self) -> String {
		format!(
			"arn:aws:iam::*:policy/{}--*--{}",
			self.app,
			RuntimeBoundary::SUFFIX
		)
	}

	/// The app every name is scoped to, ie `beet-site`.
	pub fn app(&self) -> &SmolStr { &self.app }

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
		self.stages.insert(stack.stage().clone());
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
			statements.extend(self.boundary_statements());
		}
		statements.extend(self.irreversible_statement());
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

	/// The statements that stop `iam:*` on this app's names from being a route
	/// to an administrator, absent until the app's principals are all capped.
	///
	/// Without them the `Iam` statement above is an escalation in three calls:
	/// create `role/<app>--prod--anything`, attach the AWS-managed
	/// `AdministratorAccess` to it (the resource `iam:AttachRolePolicy` checks
	/// is the role, which is in scope) and pass it to a compute the same
	/// policy may create.
	///
	/// - **create and edit only under the boundary.** `iam:PermissionsBoundary`
	///   names the boundary a `Create*` request SETS and the one an edited
	///   principal already CARRIES, so one condition both forces a new
	///   principal to be capped and refuses to touch an uncapped one.
	/// - **never uncap.** `Delete*PermissionsBoundary` is denied outright.
	/// - **never rewrite the cap.** The boundary and this policy are both
	///   `<app>--*`, which the `Iam` statement reaches, so a deployer could
	///   otherwise publish a new version of the document that grants it.
	/// - **no service-linked roles.** They can never carry a boundary at all
	///   (`PutRolePermissionsBoundary` answers `UnmodifiableEntity`), so the
	///   only cap available is not creating them.
	///
	/// `iam:PassRole` takes no boundary condition, and is bounded instead by
	/// the `Iam` statement's own `role/<app>--*` resource: with every role in
	/// that namespace capped, there is no uncapped role to pass.
	///
	/// **Every boundary deny is scoped to this app's own principals**, which is
	/// not tidiness: one deployer user carries one policy PER APP, an explicit
	/// deny is a union across all of them, and a deny on `*` would therefore
	/// make each app's policy refuse every other app's deploys, since their
	/// boundaries differ. `NeverUncap` is the deliberate exception, being a
	/// deny nothing should ever be exempt from.
	fn boundary_statements(&self) -> Vec<Value> {
		if !self.boundary {
			return Vec::new();
		}
		let boundary = self.boundary_pattern();
		vec![
			json!({
				"Sid": "IamOnlyUnderTheBoundary",
				"Effect": "Deny",
				"Action": [
					"iam:CreateRole",
					"iam:CreateUser",
					"iam:PutRolePermissionsBoundary",
					"iam:PutUserPermissionsBoundary",
					"iam:PutRolePolicy",
					"iam:PutUserPolicy",
					"iam:AttachRolePolicy",
					"iam:AttachUserPolicy",
					"iam:DeleteRolePolicy",
					"iam:DeleteUserPolicy",
					"iam:DetachRolePolicy",
					"iam:DetachUserPolicy",
				],
				// this app's principals only: see the note above about one
				// user carrying one policy per app
				"Resource": self.principal_resources(),
				// `StringNotLike`, since one policy covers every stage and
				// each stage has a boundary of its own.
				//
				// Every action here is one AWS populates `iam:PermissionsBoundary`
				// for, and that is load bearing rather than incidental: a
				// `StringNotLike` on an ABSENT key is true, so a condition on an
				// action the key is not defined for denies it outright. That is
				// why `iam:UpdateAssumeRolePolicy` is deliberately not in the
				// list -- it changes who may assume a role rather than what the
				// role may do, so AWS has no boundary to report for it, and the
				// deny would have fired on every trust-policy change as a bare
				// `AccessDenied`. It also buys nothing: the boundary is already
				// attached by then and `NeverUncap` below refuses its removal.
				"Condition": {
					"StringNotLike": { "iam:PermissionsBoundary": &boundary }
				},
			}),
			json!({
				"Sid": "NeverUncap",
				"Effect": "Deny",
				"Action": [
					"iam:DeleteRolePermissionsBoundary",
					"iam:DeleteUserPermissionsBoundary",
				],
				"Resource": self.principal_resources(),
			}),
			json!({
				"Sid": "NeverMintOrRewriteACap",
				"Effect": "Deny",
				// `CreatePolicy` belongs here because the condition above
				// admits any boundary of this app: without it a deployer
				// mints `<app>--evil--runtime-boundary` granting `*`, wears
				// it, and the cap is decorative. The mint runs as an
				// administrator, so nothing legitimate loses anything.
				"Action": [
					"iam:CreatePolicy",
					"iam:CreatePolicyVersion",
					"iam:DeletePolicy",
					"iam:DeletePolicyVersion",
					"iam:SetDefaultPolicyVersion",
				],
				"Resource": [
					&boundary,
					&format!(
						"arn:aws:iam::*:policy/{}",
						self.policy_name()
					),
				],
			}),
		]
	}

	/// The speed bump on the genuinely unrecoverable, `None` when the app's
	/// stacks render nothing that can be one.
	///
	/// Everything else this policy grants is recoverable by design: a bucket
	/// is versioned, a table has deletion protection, a noncurrent version has
	/// a window. **A deployer never needs to delete a production bucket**, so
	/// denying it costs nothing day to day and makes destroying production a
	/// human act rather than an agent's mistake.
	///
	/// `BoolIfExists`, never `Bool`: `aws:MultiFactorAuthPresent` is ABSENT
	/// from a long-lived access key's requests rather than false, and a `Bool`
	/// deny would therefore not fire for exactly the credential it is aimed
	/// at. So this denies a deployer outright, and an operator who assumes a
	/// role with a code is the only principal that gets through.
	///
	/// Scoped to the stages this launch RENDERED, plus `prod` unconditionally,
	/// one resource per stage. The rendered stages are what protects an app
	/// whose only stage is `shared`: the assets buckets are a source of record
	/// and not a mirror, and a hard-coded `--prod--` pattern named nothing that
	/// exists for them, so their own deployer could delete them with no code.
	/// Prod stays named regardless so a mint under the wrong stage cannot
	/// quietly drop the deny from the stage that matters most.
	///
	/// Tearing a DEV stack down stays free, which the infra-deploy skill does
	/// routinely, because a mint runs under the stage that deploys and `dev` is
	/// therefore never among the stages it collects. A mint run under `dev`
	/// would deny that teardown — loudly, and after the warning `DeployerMint`
	/// already prints for a non-prod stage.
	///
	/// ## What is deliberately NOT here
	///
	/// Only actions a deploy never performs, because a deny is absolute and an
	/// action a converge needs would block every apply:
	///
	/// - **`s3:PutBucketVersioning`**, though turning versioning off is how a
	///   deletion becomes final. There is no condition key for the versioning
	///   STATUS, so a deny cannot tell enabling from disabling, and every new
	///   bucket is created with versioning enabled. Recoverability is kept
	///   where it can be expressed instead: the render refuses a writable
	///   bucket that declares no versioning, and `force_destroy=false` stops
	///   the bucket going with the stack.
	/// - **`s3:PutBucketLifecycleConfiguration`**, for the same reason: a
	///   deploy sets the expiry rules on every converge.
	/// - **`iam:DeleteRole` and `iam:DeleteUser`**, because renaming a role or
	///   a user is destroy-then-create, and a rename is an ordinary change. An
	///   IAM principal is also not a source of record: it is re-mintable from
	///   the declarations, which is the whole of what makes it recoverable.
	fn irreversible_statement(&self) -> Option<Value> {
		let mut actions = Vec::<&str>::new();
		let mut resources = Vec::<String>::new();
		// PROD is in the set whatever this launch rendered. Scoping to the
		// rendered stages ALONE would mean a mint under the wrong stage dropped
		// prod from the deny, turning a wrong-stage mint from over-protecting
		// into silently under-protecting the one stage that matters. A deny
		// naming a stage an app does not have is inert.
		let stages = self
			.stages
			.iter()
			.map(SmolStr::as_str)
			.chain([BootstrapConfig::PROD_STAGE])
			.collect::<BTreeSet<_>>();
		let mut per_stage = |template: &str| {
			resources.extend(stages.iter().map(|stage| {
				template.replace("{}", &format!("{}--{stage}--*", self.app))
			}))
		};
		if self.services.contains("s3") {
			actions.push("s3:DeleteBucket");
			per_stage("arn:aws:s3:::{}");
		}
		if self.services.contains("dynamodb") {
			actions.push("dynamodb:DeleteTable");
			per_stage("arn:aws:dynamodb:*:*:table/{}");
		}
		if self.services.contains("rds") {
			actions.push("rds:DeleteDBInstance");
			per_stage("arn:aws:rds:*:*:db:{}");
		}
		(!actions.is_empty() && !resources.is_empty()).then(|| {
			json!({
				"Sid": "IrreversibleNeedsMfa",
				"Effect": "Deny",
				"Action": actions,
				"Resource": resources,
				"Condition": {
					"BoolIfExists": { "aws:MultiFactorAuthPresent": "false" }
				},
			})
		})
	}

	/// The two arn patterns naming a PRINCIPAL of this app, which is what a
	/// boundary condition applies to; the policy and instance-profile patterns
	/// of [`iam_resources`](Self::iam_resources) carry no boundary.
	fn principal_resources(&self) -> Vec<String> {
		["role", "user"]
			.into_iter()
			.map(|kind| format!("arn:aws:iam::*:{kind}/{}--*", self.app))
			.collect()
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
					S3BucketBlock::new("store")
						.with_deploy_versioned(false)
						.with_accept_data_loss(true),
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

	/// The escalation the boundary closes, and the shape that closes it: the
	/// broad `iam:*` Allow means only a DENY can restrict, so the condition is
	/// `StringNotLike` (a pattern, since one policy spans every stage), which
	/// is also true for a principal carrying no boundary at all and therefore
	/// refuses to edit an uncapped one.
	#[beet_core::test]
	fn boundary_conditions_deny_rather_than_allow() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(RepoStoreBlock::test_store());
			parent.spawn(LambdaBlock::default());
		});
		let (stack, _deployment, config) = scope.finish().unwrap();
		let policy = DeployerPolicy::new(stack.app_name().clone(), "state")
			.lower(&stack, &config)
			.unwrap();
		// unconditioned until the account says every principal is capped
		policy
			.to_json()
			.to_string()
			.xpect_contains("iam:*")
			.xnot()
			.xpect_contains("PermissionsBoundary");
		let document = policy.with_boundary().to_json();
		let statement = |sid: &str| {
			document["Statement"]
				.as_array()
				.unwrap()
				.iter()
				.find(|statement| statement["Sid"] == sid)
				.cloned()
				.unwrap()
		};
		let under = statement("IamOnlyUnderTheBoundary");
		under["Effect"].as_str().unwrap().xpect_eq("Deny");
		// a `StringNotLike` on an absent key is TRUE, so every action named
		// here must be one AWS populates `iam:PermissionsBoundary` for.
		// `UpdateAssumeRolePolicy` is not: it changes who may assume a role
		// rather than what it may do, so conditioning it would deny every
		// trust-policy change outright, and denying it buys nothing once the
		// boundary is attached.
		under["Action"]
			.to_string()
			.as_str()
			.xpect_contains("iam:CreateRole")
			.xnot()
			.xpect_contains("iam:UpdateAssumeRolePolicy");
		// `StringNotLike`, since one deploy policy covers every stage and each
		// stage carries a boundary of its own
		under["Condition"]["StringNotLike"]["iam:PermissionsBoundary"]
			.as_str()
			.unwrap()
			.xpect_eq(format!(
				"arn:aws:iam::*:policy/{}--*--runtime-boundary",
				stack.app_name()
			));
		// a capped principal cannot uncap itself, and the cap itself cannot
		// be rewritten by the policy it caps
		statement("NeverUncap")["Action"]
			.to_string()
			.xpect_contains("iam:DeleteRolePermissionsBoundary");
		// the condition admits ANY boundary of this app, so minting one is
		// what has to be denied or the cap is decorative
		let never_mint = statement("NeverMintOrRewriteACap");
		never_mint["Action"]
			.to_string()
			.xpect_contains("iam:CreatePolicy");
		never_mint["Resource"]
			.to_string()
			.xpect_contains("--*--runtime-boundary")
			.xpect_contains("--deploy");
	}

	/// One deployer user carries one policy PER APP and an explicit deny is a
	/// union across all of them, so a boundary deny on `*` would make each
	/// app's policy refuse every other app's deploys. The repo that found this
	/// has three apps on one user.
	#[beet_core::test]
	fn one_app_s_deny_does_not_reach_another_s() {
		let document = |app: &str| {
			let (scope, _dir) = RenderScope::test_render_stack(
				(
					Stack::new(app).with_stage("prod"),
					AwsRegion::new("us-west-2"),
				),
				|parent| {
					parent.spawn(RepoStoreBlock::test_store());
					parent.spawn(LambdaBlock::default());
				},
			);
			let (stack, _deployment, config) = scope.finish().unwrap();
			DeployerPolicy::new(app, "state")
				.lower(&stack, &config)
				.unwrap()
				.with_boundary()
				.to_json()
		};
		let deny = |app: &str| {
			document(app)["Statement"]
				.as_array()
				.unwrap()
				.iter()
				.find(|statement| statement["Sid"] == "IamOnlyUnderTheBoundary")
				.unwrap()
				.clone()
		};
		// the two apps a repo really carries on one user, each scoped to its
		// own principals, so neither reaches the other's
		let site = deny("beet-site");
		let social = deny("beet-social");
		for (held, mine, theirs) in [
			(&site, "beet-site", "beet-social"),
			(&social, "beet-social", "beet-site"),
		] {
			let resources = held["Resource"].to_string();
			resources
				.as_str()
				.xpect_contains(&format!("role/{mine}--*"))
				.xpect_contains(&format!("user/{mine}--*"))
				.xnot()
				.xpect_contains(theirs);
			// never `*`: a deny unions across every policy on the user, so a
			// deny on `*` makes each app's policy refuse the others' deploys
			resources.contains("\"*\"").xpect_false();
		}
		// and the `--` separator is what keeps an app named `beet` from
		// matching `beet-site--*`, which is what makes per-app scoping enough
		let beet = deny("beet")["Resource"].to_string();
		beet.as_str().xpect_contains("role/beet--*");
		beet.contains("beet-site").xpect_false();
	}

	/// Destroying production takes a human. `BoolIfExists` is the whole point:
	/// a long-lived key's requests carry no `aws:MultiFactorAuthPresent` at
	/// all, so a `Bool` deny would not fire for the credential it is aimed at.
	#[beet_core::test]
	fn irreversible_prod_actions_need_mfa() {
		let (scope, _dir) = RenderScope::test_render_stack(
			(
				Stack::new("my-egress").with_stage("prod"),
				AwsRegion::new("us-west-2"),
			),
			|parent| {
				parent.spawn(
					S3BucketBlock::new("store")
						.with_deploy_versioned(false)
						.with_accept_data_loss(true),
				);
			},
		);
		let (stack, _deployment, config) = scope.finish().unwrap();
		let document = DeployerPolicy::new("my-egress", "state")
			.lower(&stack, &config)
			.unwrap()
			.to_json();
		let deny = document["Statement"]
			.as_array()
			.unwrap()
			.iter()
			.find(|statement| statement["Sid"] == "IrreversibleNeedsMfa")
			.unwrap()
			.clone();
		deny["Effect"].as_str().unwrap().xpect_eq("Deny");
		deny["Condition"]["BoolIfExists"]["aws:MultiFactorAuthPresent"]
			.as_str()
			.unwrap()
			.xpect_eq("false");
		// only what a deploy NEVER does: `PutBucketVersioning` is how every new
		// bucket is created, and IAM has no condition key for the status, so a
		// deny there would refuse the create rather than the disable
		deny["Action"]
			.to_string()
			.xpect_contains("s3:DeleteBucket")
			.xnot()
			.xpect_contains("s3:PutBucketVersioning")
			.xnot()
			.xpect_contains("iam:DeleteRole");
		// a dev teardown stays free, which the infra-deploy skill does often
		deny["Resource"]
			.to_string()
			.xpect_contains("my-egress--prod--*")
			.xnot()
			.xpect_contains("my-egress--dev");
	}

	/// The deny follows the stages the launch RENDERED, so an app whose only
	/// stage is `shared` has its buckets protected too. A hard-coded `prod`
	/// named nothing that existed for such an app, which left the assets
	/// buckets — a source of record, not a mirror — deletable by their own
	/// deployer with no code.
	#[beet_core::test]
	fn irreversible_follows_the_rendered_stages() {
		let render = |stage: &str| {
			let (scope, dir) = RenderScope::test_render_stack(
				(
					Stack::new("beet-site").with_stage(stage),
					AwsRegion::new("us-west-2"),
				),
				|parent| {
					parent.spawn(
						S3BucketBlock::new("assets")
							.with_deploy_versioned(false)
							.with_accept_data_loss(true),
					);
				},
			);
			(scope.finish().unwrap(), dir)
		};
		let resources = |stages: &[&str]| {
			let rendered =
				stages.iter().map(|stage| render(stage)).collect::<Vec<_>>();
			rendered
				.iter()
				.fold(
					DeployerPolicy::new("beet-site", "state"),
					|policy, ((stack, _deployment, config), _dir)| {
						policy.lower(stack, config).unwrap()
					},
				)
				.to_json()["Statement"]
				.as_array()
				.unwrap()
				.iter()
				.find(|statement| statement["Sid"] == "IrreversibleNeedsMfa")
				.unwrap()["Resource"]
				.to_string()
		};
		// a shared-only app protects `shared`, where a hard-coded prod
		// protected a name the app does not have; prod stays named regardless,
		// so a wrong-stage mint cannot drop it
		resources(&["shared"])
			.as_str()
			.xpect_contains("beet-site--shared--*")
			.xpect_contains("beet-site--prod--*");
		// and an app rendering both gets one resource each
		resources(&["prod", "shared"])
			.as_str()
			.xpect_contains("beet-site--prod--*")
			.xpect_contains("beet-site--shared--*");
	}
}
