//! The recurring timer: an EventBridge schedule invoking a declared lambda with
//! the request it should dispatch.
use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

/// Run one route on a schedule: a timer, the [`LambdaBlock`] it invokes, and the
/// request that invoke dispatches.
///
/// A deployed box is destroyed and recreated by every deploy, so it can own no
/// durable timer, and a batch job has no business on a serving world's thread
/// anyway. The cloud primitive for both is a schedule invoking a function, and
/// this is its declaration: "dispatch `path` every `schedule`, on the lambda
/// this points at".
///
/// The invoke carries a [`ScheduledInvoke`] payload, so the timer names a route
/// rather than a handler and the target binary dispatches it exactly as it would
/// a request that arrived over http.
///
/// Authored directly from markup beside the lambda it drives, related through
/// [`InvokeTarget`], ie
/// ```html
/// <LambdaBlock bx:ref="rollup" label="rollup"/>
/// <ScheduledJobBlock label="rollup-daily" {InvokeTarget($rollup)}
///   schedule="cron(0 3 * * ? *)" path="analytics/rollup"/>
/// ```
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>)]
pub struct ScheduledJobBlock {
	/// The unprefixed schedule label (eg `rollup-daily`), which names the
	/// schedule and its invoke role.
	label: SmolStr,
	/// When to run, in EventBridge Scheduler's expression grammar: a `rate(1
	/// day)` interval, a six-field `cron(0 3 * * ? *)`, or a one-shot
	/// `at(2026-01-01T03:00:00)`. Validated at render, so a typo fails the
	/// deploy rather than the job.
	schedule: SmolStr,
	/// The route each invoke dispatches, optionally carrying query params, ie
	/// `analytics/rollup?full=true`.
	path: SmolStr,
	/// The method the dispatched request carries. A job acts rather than
	/// fetches, so `Post` by default; a route declaring a method only dispatches
	/// to that method.
	method: HttpMethod,
	/// The IANA timezone [`schedule`](Self::schedule) is read in. `UTC` by
	/// default: a local timezone silently moves the run twice a year.
	timezone: SmolStr,
	/// Override the region this schedule lives in, which otherwise resolves from
	/// the ancestor [`Stack`].
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
}

impl Default for ScheduledJobBlock {
	fn default() -> Self { Self::new("") }
}

/// The lambda a schedule invokes: the source half of the [`Invokers`]
/// relationship, on the [`ScheduledJobBlock`] entity, targeting a declaration
/// carrying a [`LambdaBlock`]. Authored in markup as `{InvokeTarget($rollup)}`
/// beside a `<LambdaJobBlock bx:ref="rollup"/>`.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Invokers)]
pub struct InvokeTarget(#[entities] pub Entity);

/// Every schedule invoking a lambda: the target half of the [`InvokeTarget`]
/// relationship, on the lambda's declaration entity.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = InvokeTarget)]
pub struct Invokers(Vec<Entity>);

impl ScheduledJobBlock {
	/// A schedule labelled `label`, which must be given an [`InvokeTarget`]
	/// relation, a [`schedule`](Self::schedule) expression and a
	/// [`path`](Self::path) before it renders.
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			schedule: default(),
			path: default(),
			method: HttpMethod::Post,
			timezone: Self::UTC.into(),
			region: None,
		}
	}

	/// The timezone a schedule is read in unless one is declared.
	pub const UTC: &'static str = "UTC";

	/// Build a prefixed label for this schedule's terraform resources.
	pub fn build_label(&self, suffix: &str) -> String {
		format!("{}--{suffix}", self.label)
	}

	/// The payload each invoke delivers, the request the target binary
	/// dispatches.
	pub fn invoke(&self) -> ScheduledInvoke {
		ScheduledInvoke::new(self.path.clone()).with_method(self.method)
	}

	/// The region this schedule lives in: its own override, else `stack`'s.
	pub fn resolved_region(&self, stack: &ResolvedStack) -> SmolStr {
		self.region
			.clone()
			.unwrap_or_else(|| stack.region().clone())
	}

	/// Whether this declaration can render at all: a schedule with no time to
	/// run at or no route to dispatch is a deploy-time failure rather than a
	/// timer that fires into nothing. (A schedule with nothing to invoke fails
	/// at render, where its [`InvokeTarget`] relation resolves.)
	pub fn validate(&self) -> Result {
		if self.path.is_empty() {
			bevybail!(
				"the schedule '{}' names no route to dispatch, ie `path=\"analytics/rollup\"`",
				self.label
			);
		}
		Self::validate_expression(&self.schedule)
			.map_err(|err| bevyhow!("the schedule '{}' {err}", self.label))
	}

	/// EventBridge Scheduler's expression grammar, checked at render so an
	/// unparseable expression fails the deploy rather than being accepted by
	/// tofu and rejected by the scheduler api.
	fn validate_expression(expression: &str) -> Result {
		let malformed = || {
			bevyhow!(
				"has the unparseable expression '{expression}', expected one of \
				`rate(<n> minutes|hours|days)`, a six-field `cron(0 3 * * ? *)` \
				or a one-shot `at(2026-01-01T03:00:00)`"
			)
		};
		let (kind, args) = expression
			.split_once('(')
			.and_then(|(kind, rest)| Some((kind, rest.strip_suffix(')')?)))
			.ok_or_else(malformed)?;
		match kind {
			// the scheduler accepts a singular unit only for a value of one,
			// which is a rule worth failing on here rather than at the api.
			"rate" => match args.split_whitespace().collect::<Vec<_>>()[..] {
				[value, unit]
					if value.parse::<u32>().is_ok_and(|value| value > 0)
						&& matches!(
							(unit, value),
							("minute" | "hour" | "day", "1")
								| ("minutes" | "hours" | "days", _)
						) =>
				{
					Ok(())
				}
				_ => Err(malformed()),
			},
			// minutes, hours, day-of-month, month, day-of-week, year: six, not
			// the five a unix crontab takes.
			"cron" if args.split_whitespace().count() == 6 => Ok(()),
			"at" if !args.is_empty() => Ok(()),
			_ => Err(malformed()),
		}
	}
}

impl Block for ScheduledJobBlock {
	fn label(&self) -> &SmolStr { &self.label }
}

/// The [`DeployRender`] render system, registered by [`InfraPlugin`] beside
/// the type registration.
impl ScheduledJobBlock {
	/// Render the schedule and its invoke role into the config, resolving the
	/// [`InvokeTarget`] relation to the [`LambdaBlock`] it invokes.
	pub(crate) fn render(
		mut scopes: AncestorQuery<&mut RenderScope>,
		blocks: Query<(Entity, &ScheduledJobBlock, Option<&InvokeTarget>)>,
		lambdas: Query<&LambdaBlock>,
	) {
		for (entity, block, target) in blocks.iter() {
			// skip blocks outside every rendering scope before anything errors
			if scopes.get_entity(entity).is_err() {
				continue;
			}
			let lambda = crate::types::related(
				&scopes,
				entity,
				&lambdas,
				target.map(|target| target.0),
				"InvokeTarget",
				block.label(),
			);
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			match lambda {
				Err(err) => scope.error(err),
				Ok(lambda) => {
					let (stack, _deployment, config) = scope.ctx();
					if let Err(err) = block.emit(stack, config, lambda) {
						scope.error(bevyhow!(
							"ScheduledJobBlock '{}': {err}",
							block.label()
						));
					}
				}
			}
		}
	}

	/// Emit the schedule, the invoke role scoped to the one function it targets,
	/// and that role's policy. Validates the declaration first, so a misdeclared
	/// job fails the deploy rather than rendering a timer that fires into nothing.
	fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		lambda: &LambdaBlock,
	) -> Result {
		self.validate()?;
		let region = self.resolved_region(stack);
		let function_arn = lambda.arn(stack);

		// The invoke identity: the scheduler assumes this role to call the one
		// function it targets, and can do nothing else with it. The function's
		// own role governs what the job may touch once it is running.
		let role = ResourceDef::new_primary(
			stack.resource_ident(self.build_label("invoke-role")),
			AwsIamRoleDetails {
				assume_role_policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Action": "sts:AssumeRole",
						"Effect": "Allow",
						"Principal": { "Service": "scheduler.amazonaws.com" }
					}]
				})
				.to_string()
				.into(),
				..default()
			},
		);
		let policy_ident =
			stack.resource_ident(self.build_label("invoke-policy"));
		let policy = ResourceDef::new_secondary(
			policy_ident.clone(),
			AwsIamRolePolicyDetails {
				name: Some(policy_ident.primary_identifier().clone()),
				role: role.field_ref("name").into(),
				policy: IamPolicy::new(region.clone(), "scheduled job")
					.statement(json!({
						"Sid": "InvokeTarget",
						"Effect": "Allow",
						"Action": "lambda:InvokeFunction",
						"Resource": function_arn
					}))
					.render()
					.into(),
				..default()
			},
		);

		let ident = stack.resource_ident(self.label.clone());
		let schedule = ResourceDef::new_secondary(
			ident.clone(),
			AwsSchedulerScheduleDetails {
				name: Some(ident.primary_identifier().clone()),
				schedule_expression: self.schedule.clone(),
				schedule_expression_timezone: Some(self.timezone.clone()),
				// `OFF` runs at the declared time. A flexible window exists to
				// spread load across many schedules, which is not what a single
				// nightly job wants.
				flexible_time_window: Some(vec![
					AwsSchedulerScheduleResourceBlockTypeFlexibleTimeWindow {
						mode: "OFF".into(),
						..default()
					},
				]),
				region: Some(region),
				target: Some(vec![
					AwsSchedulerScheduleResourceBlockTypeTarget {
						arn: function_arn.into(),
						role_arn: role.field_ref("arn").into(),
						// the request, rendered by the one type the adapter
						// deserializes, so tofu never hand-crafts an event
						input: Some(
							serde_json::to_string(&self.invoke())?.into(),
						),
						..default()
					},
				]),
				..default()
			},
		);

		config
			.add_resource(&role)?
			.add_resource(&policy)?
			.add_resource(&schedule)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The rendered json for a valid schedule related to the `rollup` lambda.
	fn build_json(block: &ScheduledJobBlock) -> Result<String> {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			let lambda = parent
				.spawn(LambdaBlock::default().with_label("rollup"))
				.id();
			parent.spawn((block.clone(), InvokeTarget(lambda)));
		});
		let (_stack, _deployment, config) = scope.finish()?;
		config.to_json_string()
	}

	fn rollup_daily() -> ScheduledJobBlock {
		ScheduledJobBlock::new("rollup-daily")
			.with_schedule("cron(0 3 * * ? *)")
			.with_path("analytics/rollup")
	}

	/// The declaration renders a schedule, an invoke role scoped to the one
	/// function it targets, and the payload that function dispatches.
	#[beet_core::test]
	fn emits_a_schedule_and_its_invoke_role() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let json = build_json(&rollup_daily()).unwrap();
		json.as_str()
			.xpect_contains("aws_scheduler_schedule")
			.xpect_contains(&stack.resource_name("rollup-daily"))
			.xpect_contains("cron(0 3 * * ? *)")
			.xpect_contains("\"schedule_expression_timezone\":\"UTC\"")
			.xpect_contains("\"mode\":\"OFF\"")
			.xpect_contains("scheduler.amazonaws.com")
			.xpect_contains("lambda:InvokeFunction")
			// the payload the adapter deserializes, escaped into the target input
			.xpect_contains("beet.scheduled_invoke.v1")
			.xpect_contains("analytics/rollup");
		// the invoke role reaches exactly one function, never a wildcard
		json.as_str()
			.xpect_contains(&format!(
				"aws_lambda_function.{}.arn",
				stack.resource_ident("rollup--function").label()
			))
			.xnot()
			.xpect_contains("\"Resource\":\"*\"");
	}

	/// The schedule targets the ident the lambda block emits, composed by the
	/// one block both sides go through.
	#[beet_core::test]
	fn targets_the_ident_the_lambda_emits() {
		let lambda = LambdaBlock::default().with_label("rollup");
		let (scope, _dir) = RenderScope::test_render(|parent| {
			let target = parent
				.spawn(LambdaBlock::default().with_label("rollup"))
				.id();
			parent.spawn((rollup_daily(), InvokeTarget(target)));
		});
		let (stack, _deployment, config) = scope.finish().unwrap();
		// the schedule's interpolation names a resource this config declares
		let json = config.to_json_string().unwrap();
		let address =
			format!("aws_lambda_function.{}", lambda.ident(&stack).label());
		json.matches(&address).count().xpect_greater_than(1);
	}

	/// The rendered schedule is a pure function of the DECLARATION: two deploys
	/// render it byte-identically, so a plan shows no diff when nothing changed
	/// and a deploy-scoped value cannot leak into the payload unnoticed. (The
	/// target lambda legitimately carries the deploy id in its artifact key, so
	/// the comparison is the schedule's resource alone.)
	#[beet_core::test]
	fn renders_deterministically() {
		let schedule = || {
			serde_json::from_str::<serde_json::Value>(
				&build_json(&rollup_daily()).unwrap(),
			)
			.unwrap()["resource"]["aws_scheduler_schedule"]
				.clone()
		};
		schedule().xpect_eq(schedule());
	}

	/// An unparseable expression fails the DEPLOY, naming the schedule and the
	/// grammar. A schedule the api rejects is a job that silently never runs.
	#[beet_core::test]
	fn an_invalid_expression_fails_the_deploy() {
		for expression in [
			"every day",
			"cron(0 3 * * *)",
			"rate(0 days)",
			"rate(2 day)",
			"rate(day)",
			"at()",
			"weekly(1)",
		] {
			build_json(&rollup_daily().with_schedule(expression))
				.unwrap_err()
				.to_string()
				.xpect_contains("rollup-daily")
				.xpect_contains(expression);
		}
		for expression in [
			"cron(0 3 * * ? *)",
			"rate(1 day)",
			"rate(30 minutes)",
			"at(2026-01-01T03:00:00)",
		] {
			build_json(&rollup_daily().with_schedule(expression)).unwrap();
		}
	}

	/// The provider accepts what this block renders, which no amount of
	/// contains-assertions can prove: a misnamed nested block or a field the
	/// schema does not carry only surfaces at apply otherwise. Rendered with the
	/// lambda it targets, since the invoke reference must resolve.
	// drives the native tofu Project, so it cannot compile for wasm
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			let target = parent
				.spawn(LambdaBlock::default().with_label("rollup"))
				.id();
			parent.spawn((rollup_daily(), InvokeTarget(target)));
		});
		scope.project().unwrap().validate().await.unwrap();
	}

	/// A schedule with nothing to invoke, a target that is not a lambda, or
	/// nothing to dispatch fails naming what it is missing rather than rendering
	/// a timer that fires into nothing.
	#[beet_core::test]
	fn an_unpointed_schedule_fails_the_deploy() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(rollup_daily());
		});
		scope
			.finish()
			.unwrap_err()
			.to_string()
			.xpect_contains("declares no `InvokeTarget`")
			.xpect_contains("rollup-daily");
		// a target carrying no LambdaBlock is a dangling reference, not a timer
		let (scope, _dir) = RenderScope::test_render(|parent| {
			let target = parent.spawn(()).id();
			parent.spawn((rollup_daily(), InvokeTarget(target)));
		});
		scope
			.finish()
			.unwrap_err()
			.to_string()
			.xpect_contains("carries no `LambdaBlock`");
		build_json(&rollup_daily().with_path(""))
			.unwrap_err()
			.to_string()
			.xpect_contains("names no route to dispatch");
	}
}
