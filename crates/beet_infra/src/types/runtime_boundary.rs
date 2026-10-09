//! The ceiling on what a deploy of an app may create, LOWERED from the IAM
//! policy documents its stacks render.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// The third policy in the model, beside [`DeployerPolicy`] (what the operator
/// may do) and [`IamPolicy`] (what one running process may do): the
/// **permissions boundary**, a managed policy attached to a principal AS its
/// boundary, capping what that principal can ever do whatever its own policies
/// say. One per app and STAGE, named `<app>--<stage>--runtime-boundary`.
///
/// ## What it is for
///
/// A deployer holds `iam:*` on its own app's names, which bounds what it can
/// REACH but not what it can GRANT: three calls create `role/<app>--prod--x`,
/// attach the AWS-managed `AdministratorAccess` to it (the resource
/// `iam:AttachRolePolicy` checks is the role, which is in scope) and pass it to
/// a compute the same policy may create. The boundary closes that, because the
/// effective permissions of a boundaried principal are the INTERSECTION of its
/// own policies and the boundary, and this one grants no `iam:` action at all.
///
/// ## What it holds, and why it is derived rather than written
///
/// The union of every IAM policy document the app's stacks render: each
/// `aws_iam_role_policy` and `aws_iam_user_policy` body ([`IamPolicy`]'s
/// lowering of the stack's declared grants plus whatever each compute seeds for
/// itself), plus the AWS-managed policies those principals attach. So it says
/// exactly "no principal this app creates may ever exceed this app's runtime
/// needs", and it says it in the same words the runtime identities do.
///
/// Deriving it from the RENDER rather than from the account is what makes the
/// superset check meaningful: the render is the configuration, the account is
/// what past applies produced, and a diff between them is a real finding.
///
/// A managed policy AWS owns is mirrored verbatim, `Resource: "*"` included.
/// Narrowing `AWSLambdaBasicExecutionRole`'s `logs:*` to the app's own groups
/// would be tighter and is a change to prove against a live runtime rather than
/// to guess: an intersection is silent, so a boundary one pattern short of what
/// a function needs stops its logs with no error anywhere.
///
/// ## The cost, which is real
///
/// A grant added to a stack widens the rendered runtime policy but not this,
/// until the next elevated deploy. The apply succeeds and the runtime silently
/// cannot use the new permission. That is the same cadence [`DeployerPolicy`]
/// already has, since a new SERVICE needs a re-mint too, and it is why the mint
/// converges both policies from one render.
#[derive(Debug, Clone)]
pub struct RuntimeBoundary {
	/// The app the boundary caps, ie `beet-site`.
	app: SmolStr,
	/// The stage, since the content is one stage's and the names it grants are
	/// that stage's alone.
	stage: SmolStr,
	/// The unioned statements, in render order, deduplicated by everything but
	/// their `Sid`.
	statements: Vec<Value>,
}

impl RuntimeBoundary {
	/// The resource types whose body carries an inline policy document.
	const INLINE_POLICY_TYPES: &'static [&'static str] =
		&["aws_iam_role_policy", "aws_iam_user_policy"];

	/// Attributes of a principal that grant it permissions without a separate
	/// resource, which this lowering does not read. No block sets one today,
	/// and one that starts to must fail here rather than earn a boundary short
	/// of its own policy.
	const UNREAD_GRANT_ATTRIBUTES: &'static [&'static str] =
		&["inline_policy", "managed_policy_arns"];

	/// The resource types that attach a managed policy to a principal.
	const ATTACHMENT_TYPES: &'static [&'static str] = &[
		"aws_iam_role_policy_attachment",
		"aws_iam_user_policy_attachment",
	];

	/// Every AWS-managed policy a beet block attaches, and the actions it
	/// grants, transcribed from the live documents on 2026-09-28. Both grant
	/// `Resource: "*"`, so the boundary mirrors that rather than narrowing it.
	///
	/// A managed policy this does not name FAILS the lowering, the same
	/// loud-on-unknown rule the rest of the derivation has: a boundary missing
	/// an attached policy's actions caps the principal below its needs, and an
	/// intersection fails silently.
	const MANAGED: &'static [(&'static str, &'static [&'static str])] = &[
		(
			"arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
			&[
				"logs:CreateLogGroup",
				"logs:CreateLogStream",
				"logs:PutLogEvents",
			],
		),
		(
			"arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy",
			&[
				"ecr:GetAuthorizationToken",
				"ecr:BatchCheckLayerAvailability",
				"ecr:GetDownloadUrlForLayer",
				"ecr:BatchGetImage",
				"logs:CreateLogStream",
				"logs:PutLogEvents",
			],
		),
	];

	/// The arn shape of every resource type a rendered policy may REFER to
	/// rather than name: `(terraform type, service, arn resource kind)`.
	///
	/// A rendered document may carry a terraform reference where an arn goes
	/// (`${aws_lambda_function.x.arn}`, the scheduler's invoke target), which
	/// tofu resolves at apply and a policy created outside tofu cannot. The
	/// reference names a resource whose label maps to its AWS name by one
	/// rule ([`terra::Ident`]: snake label, kebab identifier), so the arn is
	/// recoverable here.
	const ARN_SHAPES: &'static [(&'static str, &'static str, &'static str)] =
		&[("aws_lambda_function", "lambda", "function")];

	/// A boundary for one stage of `app`.
	pub fn new(app: impl Into<SmolStr>, stage: impl Into<SmolStr>) -> Self {
		Self {
			app: app.into(),
			stage: stage.into(),
			statements: Vec::new(),
		}
	}

	/// The suffix every boundary policy's name ends in, and the wildcard a
	/// deploy policy's condition matches them all by.
	pub const SUFFIX: &'static str = "runtime-boundary";

	/// The managed policy this lowering writes, ie
	/// `beet-site--prod--runtime-boundary`.
	///
	/// **Per stage, not per app**, unlike the deploy policy beside it. A
	/// launch renders ONE stage, so the names a boundary grants are that
	/// stage's: a prod-derived boundary worn by a dev role would intersect
	/// with the dev role's own policy to nothing, and the dev process would
	/// fail to read its own bucket with no error anywhere. The deploy policy
	/// can be per app because every name it scopes is stage-independent; a
	/// boundary's content is not.
	pub fn policy_name(&self) -> String {
		format!("{}--{}--{}", self.app, self.stage, Self::SUFFIX)
	}

	/// The boundary arn every principal this stack renders must carry,
	/// declaring the `aws_caller_identity` data source it composes with. One
	/// call per principal, so a block cannot name the boundary without also
	/// declaring what resolves the account id.
	///
	/// A boundary is named by a full arn, so unlike the wildcarded account
	/// segment a rendered policy uses, this one has to resolve. The account is
	/// not known at render time and tofu resolves the reference at apply.
	pub fn arn(
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result<SmolStr> {
		config.add_untyped_data_source(
			"aws_caller_identity",
			"current",
			&json!({}),
		)?;
		format!(
			"arn:aws:iam::${{data.aws_caller_identity.current.account_id}}:policy/{}",
			Self::new(stack.app_name().clone(), stack.stage().clone())
				.policy_name()
		)
		.xmap(SmolStr::new)
		.xok()
	}

	/// The principal types a deploy creates, and the attribute naming each.
	const PRINCIPAL_TYPES: &'static [(&'static str, &'static str)] =
		&[("aws_iam_role", "role"), ("aws_iam_user", "user")];

	/// Every principal `config` renders, as `(kind, name)` where kind is
	/// `role` or `user`: what a mint asks the account about to learn whether
	/// this app's deploys are capped yet, which is what decides whether its
	/// deploy policy may carry the boundary conditions.
	pub fn principals(
		config: &terra::Config,
	) -> Result<Vec<(&'static str, SmolStr)>> {
		let mut principals = Vec::new();
		for (resource_type, kind) in Self::PRINCIPAL_TYPES {
			for (label, body) in config.resources_of_type(resource_type) {
				principals.push((
					*kind,
					body.get("name")
						.and_then(|name| name.as_str().ok())
						.ok_or_else(|| {
							bevyhow!(
								"`{resource_type}.{label}` renders no `name`, \
								so nothing can ask the account whether it is \
								capped"
							)
						})?
						.xmap(SmolStr::new),
				));
			}
		}
		principals.xok()
	}

	/// Whether the app's stacks render no principal at all, in which case there
	/// is nothing to cap and no policy to create: an empty `Statement` list is
	/// not a valid IAM document.
	pub fn is_empty(&self) -> bool { self.statements.is_empty() }

	/// Lower one of the app's stacks: every inline policy its blocks rendered
	/// and every managed policy they attach.
	pub fn lower(mut self, config: &terra::Config) -> Result<Self> {
		for (resource_type, _) in Self::PRINCIPAL_TYPES {
			for (label, body) in config.resources_of_type(resource_type) {
				if let Some(attribute) = Self::UNREAD_GRANT_ATTRIBUTES
					.iter()
					.find(|attribute| body.get(attribute).is_some())
				{
					bevybail!(
						"`{resource_type}.{label}` grants through \
						`{attribute}`, which this lowering does not read, so \
						the boundary would cap the principal below its own \
						policy and every call it misses would fail silently. \
						Render the grant as an `{resource_type}_policy` \
						resource, or teach `RuntimeBoundary::lower` to read \
						the attribute"
					);
				}
			}
		}
		for resource_type in Self::INLINE_POLICY_TYPES {
			for (label, body) in config.resources_of_type(resource_type) {
				let document = body
					.get("policy")
					.and_then(|policy| policy.as_str().ok())
					.ok_or_else(|| {
						bevyhow!(
							"`{resource_type}.{label}` carries no `policy` \
							document, so the boundary cannot tell what it grants"
						)
					})?;
				for statement in
					serde_json::from_str::<Value>(document)?["Statement"]
						.as_array()
						.ok_or_else(|| {
							bevyhow!(
								"`{resource_type}.{label}`'s policy has no \
								`Statement` list"
							)
						})?
						.clone()
				{
					self.push(
						Self::sid(label),
						Self::resolve(config, statement)?,
					);
				}
			}
		}
		for resource_type in Self::ATTACHMENT_TYPES {
			for (label, body) in config.resources_of_type(resource_type) {
				let arn = body
					.get("policy_arn")
					.and_then(|arn| arn.as_str().ok())
					.ok_or_else(|| {
						bevyhow!(
							"`{resource_type}.{label}` carries no `policy_arn`"
						)
					})?;
				let actions = Self::MANAGED
					.iter()
					.find(|(managed, _)| *managed == arn)
					.map(|(_, actions)| *actions)
					.ok_or_else(|| {
						bevyhow!(
							"`{resource_type}.{label}` attaches `{arn}`, whose \
							actions are not declared in \
							`RuntimeBoundary::MANAGED`: a boundary short of an \
							attached policy caps the principal below its needs, \
							and an intersection fails silently"
						)
					})?;
				self.push(
					Self::sid(label),
					json!({
						"Effect": "Allow",
						"Action": actions,
						"Resource": "*",
					}),
				);
			}
		}
		self.xok()
	}

	/// The policy document, ready for `iam create-policy`.
	pub fn to_json(&self) -> Value {
		json!({ "Version": "2012-10-17", "Statement": self.statements })
	}

	/// Add `statement` under a `Sid` derived from `prefix`, unless an identical
	/// statement is already held: two principals of one app routinely read the
	/// same bucket, and the union is a set.
	fn push(&mut self, prefix: String, mut statement: Value) {
		let sid = format!(
			"{prefix}{}",
			statement
				.get("Sid")
				.and_then(Value::as_str)
				.unwrap_or_default()
		);
		statement.as_object_mut().map(|object| object.remove("Sid"));
		if self
			.statements
			.iter()
			.any(|held| Self::same(held, &statement))
		{
			return;
		}
		statement["Sid"] = Value::String(self.unique_sid(sid));
		self.statements.push(statement);
	}

	/// Whether two statements grant the same thing, their `Sid` aside.
	fn same(left: &Value, right: &Value) -> bool {
		let without_sid = |statement: &Value| {
			let mut statement = statement.clone();
			statement.as_object_mut().map(|object| object.remove("Sid"));
			statement
		};
		without_sid(left) == without_sid(right)
	}

	/// A `Sid` fragment from a resource label: alphanumeric only, as IAM
	/// requires, and with the app's own prefix dropped since every statement
	/// here carries it.
	fn sid(label: &str) -> String {
		label
			.split("__")
			.skip(1)
			.flat_map(|part| part.split('_'))
			.map(|part| {
				let mut chars = part.chars();
				match chars.next() {
					Some(first) => {
						first.to_uppercase().collect::<String>()
							+ &chars
								.filter(char::is_ascii_alphanumeric)
								.collect::<String>()
					}
					None => String::new(),
				}
			})
			.collect()
	}

	/// `statement` with every terraform reference in its `Resource` replaced by
	/// the arn tofu resolves it to, see [`ARN_SHAPES`](Self::ARN_SHAPES).
	fn resolve(config: &terra::Config, mut statement: Value) -> Result<Value> {
		let Some(resource) = statement.get("Resource") else {
			return statement.xok();
		};
		let resolved = match resource {
			Value::String(one) => {
				Value::String(Self::resolve_arn(config, one)?)
			}
			Value::Array(many) => Value::Array(
				many.iter()
					.map(|one| match one.as_str() {
						Some(one) => {
							Self::resolve_arn(config, one).map(Value::String)
						}
						None => one.clone().xok(),
					})
					.collect::<Result<Vec<_>>>()?,
			),
			other => other.clone(),
		};
		statement["Resource"] = resolved;
		statement.xok()
	}

	/// One arn, with a `${aws_type.label.attr}` reference replaced by the arn
	/// that resource carries. Fails naming a reference this cannot resolve,
	/// rather than emitting a document AWS refuses.
	fn resolve_arn(config: &terra::Config, arn: &str) -> Result<String> {
		let Some(inner) =
			arn.strip_prefix("${").and_then(|arn| arn.strip_suffix('}'))
		else {
			// an arn that merely CONTAINS a reference, ie
			// `format!("{}/*", bucket.field_ref("arn"))`: the resolution below
			// understands a whole-string one only, and a half-resolved arn in
			// a policy created outside tofu is a document AWS refuses
			if arn.contains("${") {
				bevybail!(
					"a rendered policy points at `{arn}`, which embeds a \
					terraform reference in a larger string: only a whole-string \
					reference resolves here, so render the arn with a literal \
					name"
				);
			}
			return arn.to_string().xok();
		};
		let mut parts = inner.split('.');
		let (Some(resource_type), Some(label)) = (parts.next(), parts.next())
		else {
			bevybail!("`{arn}` is not a `${{type.label.attribute}}` reference");
		};
		let (_, service, kind) = Self::ARN_SHAPES
			.iter()
			.find(|(declared, ..)| *declared == resource_type)
			.ok_or_else(|| {
				bevyhow!(
					"a rendered policy points at `{arn}`, and no arn shape is \
					declared for `{resource_type}`: add one to \
					`RuntimeBoundary::ARN_SHAPES`"
				)
			})?;
		if config
			.resources_of_type(resource_type)
			.all(|(held, _)| held != label)
		{
			bevybail!(
				"a rendered policy points at `{arn}`, which this stack does not \
				declare"
			);
		}
		// `terra::Ident` derives the config label and the AWS name from the same
		// parts, snake and kebab, so one is the other with its separators swapped
		format!("arn:aws:{service}:*:*:{kind}:{}", label.replace('_', "-"))
			.xok()
	}

	/// `sid` with a trailing ordinal when an earlier statement already took
	/// it: a `Sid` must be unique within one document, and the label-plus-Sid
	/// concatenation drops its separators, so `a_b` + `C` and `a` + `BC` both
	/// read `ProdABC`. A duplicate is `MalformedPolicyDocument` at the
	/// account, mid-mint.
	fn unique_sid(&self, sid: String) -> String {
		let taken = |candidate: &str| {
			self.statements.iter().any(|held| {
				held.get("Sid").and_then(Value::as_str) == Some(candidate)
			})
		};
		if !taken(&sid) {
			return sid;
		}
		(2..)
			.map(|ordinal| format!("{sid}{ordinal}"))
			.find(|candidate| !taken(candidate))
			.unwrap_or(sid)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A lambda's boundary: the grants its runtime policy lowered, the actions
	/// its managed policy attaches, and nothing else. In particular no `iam:`
	/// action, which is the whole point.
	#[beet_core::test]
	fn lowers_a_lambda_stack() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(RepoStoreBlock::test_store());
			parent.spawn(LambdaBlock::default());
			parent.spawn(
				S3BucketBlock::new("app")
					.with_deploy_versioned(false)
					.with_accept_data_loss(true),
			);
		});
		let (stack, _deployment, config) = scope.finish().unwrap();
		let boundary = RuntimeBoundary::new(
			stack.app_name().clone(),
			stack.stage().clone(),
		)
		.lower(&config)
		.unwrap();
		boundary.is_empty().xpect_false();
		// per stage, since the names it grants are that stage's
		boundary.policy_name().xpect_eq(format!(
			"{}--{}--runtime-boundary",
			stack.app_name(),
			stack.stage()
		));
		boundary
			.to_json()
			.to_string()
			.as_str()
			// what the managed `AWSLambdaBasicExecutionRole` grants, which an
			// intersection would otherwise silence
			.xpect_contains("logs:PutLogEvents")
			// what the declared bucket grant lowered to
			.xpect_contains(&format!(
				"arn:aws:s3:::{}",
				stack.resource_name("app")
			))
			.xnot()
			.xpect_contains("iam:");
	}

	/// A stack with no principal caps nothing, so no policy is written: an
	/// empty `Statement` list is not a valid IAM document.
	#[beet_core::test]
	fn a_bucket_only_stack_has_no_boundary() {
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
		let (_stack, _deployment, config) = scope.finish().unwrap();
		RuntimeBoundary::new("my-egress", "prod")
			.lower(&config)
			.unwrap()
			.is_empty()
			.xpect_true();
	}

	/// The union is a set: two principals of one app routinely read the same
	/// bucket, and the boundary says so once.
	#[beet_core::test]
	fn identical_statements_render_once() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(RepoStoreBlock::test_store());
			parent.spawn(LambdaBlock::default());
			parent.spawn(
				S3BucketBlock::new("app")
					.with_deploy_versioned(false)
					.with_accept_data_loss(true),
			);
		});
		let (_stack, _deployment, config) = scope.finish().unwrap();
		let once = RuntimeBoundary::new("beet-site", "prod")
			.lower(&config)
			.unwrap();
		// lowering the same render twice adds nothing
		let twice = once.clone().lower(&config).unwrap();
		twice
			.to_json()
			.to_string()
			.xpect_eq(once.to_json().to_string());
	}

	/// A rendered document may point at a resource rather than name it, which
	/// tofu resolves at apply and a policy created outside tofu cannot. The
	/// reference is resolved here, and an undeclared shape fails loudly rather
	/// than emitting a document AWS refuses.
	#[beet_core::test]
	fn resolves_a_terraform_reference() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(RepoStoreBlock::test_store());
			let lambda = parent
				.spawn(LambdaBlock::default().with_label("rollup"))
				.id();
			parent.spawn((
				ScheduledJobBlock::new("rollup-daily")
					.with_schedule("cron(0 3 * * ? *)")
					.with_path("analytics/rollup"),
				InvokeTarget(lambda),
			));
		});
		let (stack, _deployment, config) = scope.finish().unwrap();
		let document = RuntimeBoundary::new(
			stack.app_name().clone(),
			stack.stage().clone(),
		)
		.lower(&config)
		.unwrap()
		.to_json()
		.to_string();
		document
			.as_str()
			.xpect_contains("lambda:InvokeFunction")
			.xpect_contains("arn:aws:lambda:*:*:function:")
			// the reference itself never reaches the document
			.xnot()
			.xpect_contains("${");
	}

	/// A managed policy no beet block is known to attach fails the lowering:
	/// a boundary short of an attached policy's actions caps the principal
	/// below its needs, and an intersection fails silently.
	#[beet_core::test]
	fn an_unknown_managed_policy_fails() {
		let mut config = terra::Config::new();
		config
			.add_untyped_resource(
				"aws_iam_role_policy_attachment",
				"app__prod__thing",
				&serde_json::json!({
					"role": "x",
					"policy_arn": "arn:aws:iam::aws:policy/AdministratorAccess",
				}),
			)
			.unwrap();
		RuntimeBoundary::new("app", "prod")
			.lower(&config)
			.unwrap_err()
			.to_string()
			.xpect_contains("AdministratorAccess")
			.xpect_contains("RuntimeBoundary::MANAGED");
	}

	/// A `Sid` must be unique within one document, so a statement's own is
	/// prefixed by the resource it came from, with the app's name dropped.
	#[beet_core::test]
	fn sids_are_unique_and_readable() {
		let mut config = terra::Config::new();
		for label in ["app__prod__one_policy", "app__prod__two_policy"] {
			config
				.add_untyped_resource(
					"aws_iam_role_policy",
					label,
					&serde_json::json!({
						"role": "x",
						"policy": serde_json::json!({
							"Version": "2012-10-17",
							"Statement": [{
								"Sid": "ReadStores",
								"Effect": "Allow",
								"Action": "s3:GetObject",
								"Resource": format!("arn:aws:s3:::{label}"),
							}],
						}).to_string(),
					}),
				)
				.unwrap();
		}
		let document = RuntimeBoundary::new("app", "prod")
			.lower(&config)
			.unwrap()
			.to_json();
		document["Statement"]
			.as_array()
			.unwrap()
			.iter()
			.map(|statement| statement["Sid"].as_str().unwrap().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"ProdOnePolicyReadStores".to_string(),
				"ProdTwoPolicyReadStores".to_string(),
			]);
		// and the document is a valid one: every Sid distinct
		document["Statement"]
			.as_array()
			.unwrap()
			.iter()
			.filter_map(|statement| statement["Sid"].as_str())
			.collect::<std::collections::BTreeSet<_>>()
			.len()
			.xpect_eq(2);
	}
}
