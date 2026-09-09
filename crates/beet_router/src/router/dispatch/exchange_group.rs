//! Group-run routes: a route whose endpoint is a [`RunGroup`], served through an
//! [`ExchangeOverload`] rather than a bespoke wrapper action.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker for a route that runs a [`Group`], forward or reversed:
///
/// ```bsx
/// <Route path="deploy" {(ExchangeGroup, RunGroup{group: $up})}/>
/// <Route path="destroy" {(ExchangeGroup{force_param:"force"}, RunGroup{group: $down, reverse: true})}/>
/// ```
///
/// The sibling of [`ExchangeSequence`], and the same shell: route dispatch
/// reaches the run through the required [`ExchangeOverload`], mapping `Pass` to
/// `200` and `Fail` to the failing step's response, and members that carry no
/// action at all (a config block) or a differently-shaped one are skipped via
/// [`BypassErrors`]. What differs is where the steps come from: a sequence runs
/// its children, a group runs its [`Members`], so the steps are authored where
/// they belong rather than under the verb that runs them.
///
/// [`NONE_VALID`](ChildError::NONE_VALID) is deliberately NOT bypassed, for the
/// reason it is not bypassed there either: a run that skipped every member ran
/// nothing, and a `200` for work that never happened is the one outcome worse
/// than a failure. An empty group is a different thing and stays a clean no-op.
///
/// The [`RunGroup`] the route dispatches is taken by value at call time, so the
/// run mode can be decided per request: `force_param` names the flag that
/// switches it to continue-on-failure, which is what `--force` means on a
/// teardown.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	BypassErrors = BypassErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
	RunGroup<Request, Response>,
	ExchangeOverload = group_overload(),
)]
pub struct ExchangeGroup {
	/// The request param that switches this run into continue-on-failure mode,
	/// ie `force` on a teardown. Absent, the run is whatever the [`RunGroup`]
	/// declared.
	pub force_param: Option<SmolStr>,
}

impl ExchangeGroup {
	/// A route whose `param` flag switches the run into continue-on-failure.
	pub fn forced_by(param: impl Into<SmolStr>) -> Self {
		Self {
			force_param: Some(param.into()),
		}
	}
}

/// The [`ExchangeOverload`] serving dispatch from a [`RunGroup<Request, Response>`]:
/// `Pass` becomes a `200`, `Fail` the failing step's response.
fn group_overload() -> ExchangeOverload {
	ActionOverload::new(Action::new_async(
		async |cx: ActionContext<Request>| -> Result<Response> {
			let forced = cx
				.caller
				.get(|shell: &ExchangeGroup| shell.force_param.clone())
				.await?
				.is_some_and(|param| cx.input.has_param(&param));
			// by value, so the request's flag decides this run's mode without
			// editing the route's declaration.
			let mut run = cx
				.caller
				.get(|run: &RunGroup<Request, Response>| run.clone())
				.await?;
			run.continue_on_failure |= forced;
			let caller = cx.caller.clone();
			match caller.call_detached(run.into_action(), cx.input).await {
				Ok(Pass(_)) => Response::ok().xok(),
				Ok(Fail(response)) => response.xok(),
				Err(err) => Err(name_unregistered_members(&caller, err).await),
			}
		},
	))
}

/// Append the names of any [`UnregisteredTag`] members to a failed run, the
/// group counterpart of the sequence's descendant sweep: a group's steps are
/// authored anywhere, so the tags that did not link are found through
/// [`Members`] rather than through the hierarchy.
async fn name_unregistered_members(
	caller: &AsyncEntity,
	err: BevyError,
) -> BevyError {
	caller
		.with_state::<(
			Query<&UnregisteredTag>,
			Query<&Members>,
			Query<&RunGroup<Request, Response>>,
		), _>(|entity, (tags, members, runs)| {
			runs.get(entity)
				.ok()
				.and_then(|run| run.group.clone().into_inner())
				.and_then(|group| members.get(group).ok())
				.map(|members| {
					members
						.iter()
						.filter_map(|member| tags.get(member).ok())
						.map(|tag| format!("`<{}>`", tag.as_str()))
						.collect::<Vec<_>>()
				})
				.unwrap_or_default()
		})
		.await
		.ok()
		.filter(|tags| !tags.is_empty())
		.map(|tags| {
			bevyhow!(
				"{err}\nnote: this binary did not register: {}",
				tags.join(", ")
			)
		})
		.unwrap_or(err)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	fn passing_step() -> Action<Request, Outcome<Request, Response>> {
		Action::new_pure(|cx: ActionContext<Request>| {
			Outcome::Pass(cx.take()).xok()
		})
	}
	fn failing_step() -> Action<Request, Outcome<Request, Response>> {
		Action::new_pure(|_: ActionContext<Request>| {
			Outcome::<Request, Response>::Fail(Response::from_status(
				StatusCode::IM_A_TEAPOT,
			))
			.xok()
		})
	}

	#[beet_core::test]
	async fn a_passing_group_responds_ok() {
		let mut world = router_world();
		let group = world
			.spawn((Group, children![passing_step(), passing_step()]))
			.flush();
		world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeGroup::default(),
				RunGroup::<Request, Response>::new(group),
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn a_failing_member_returns_its_response() {
		let mut world = router_world();
		let group = world
			.spawn((Group, children![
				passing_step(),
				failing_step(),
				passing_step()
			]))
			.flush();
		world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeGroup::default(),
				RunGroup::<Request, Response>::new(group),
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	/// The declared flag switches the run mode per request, so the same route
	/// serves both `destroy` and `destroy --force`.
	#[beet_core::test]
	async fn the_force_param_continues_past_a_failure() {
		let mut world = router_world();
		let group = world
			.spawn((Group, children![failing_step(), passing_step()]))
			.flush();
		let router = world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeGroup::forced_by("force"),
				RunGroup::<Request, Response>::new(group),
			)]))
			.flush();
		// without the flag the failing member's response is the answer
		world
			.entity_mut(router)
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
		// with it the run finishes, then reports what failed
		world
			.entity_mut(router)
			.exchange(Request::get("run?force"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("1 failed");
	}

	/// A run over members that are all inert — the shape a lean binary loads an
	/// undeclared deploy verb as — fails rather than serving a `200`, and names
	/// the tags that did not link.
	#[beet_core::test]
	async fn inert_members_fail_loudly() {
		let mut world = router_world();
		let group = world
			.spawn((Group, children![UnregisteredTag::new("TofuApply")]))
			.flush();
		world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("deploy"),
				ExchangeGroup::default(),
				RunGroup::<Request, Response>::new(group),
			)]))
			.exchange(Request::get("deploy"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("skipped all 1 of its steps")
			.xpect_contains("this binary did not register: `<TofuApply>`");
	}
}
