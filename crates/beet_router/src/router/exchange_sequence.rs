//! Child-sequenced routes: a route whose children run as a [`Sequence`],
//! served through a [`ExchangeOverload`] rather than a bespoke wrapper action.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker for a route whose children run as a request-threading [`Sequence`]:
///
/// ```bsx
/// <Route path="deploy" {ExchangeSequence}>
///     <SomeConfig/>
///     <SomeAction/>
/// </Route>
/// ```
///
/// A thin shell over [`Sequence<Request, Response>`], whose canonical action is
/// `Request -> Outcome<Request, Response>`. Route dispatch reaches it through the
/// required [`ExchangeOverload`], mapping `Pass` to `200` and `Fail` to the failing
/// step's response. The request threads child to child, and children with no
/// action at all (config blocks) or a differently-shaped one are skipped via
/// [`BypassErrors`]; a step that is natively another shape carries its own
/// [`ActionOverload`].
///
/// [`NONE_VALID`](ChildError::NONE_VALID) is deliberately NOT bypassed: a route
/// that skipped every child ran nothing, and a `200` for work that never
/// happened is the one outcome worse than a failure. A failed run over
/// children with [`UnregisteredTag`] markers appends their names, so a lean
/// binary's error explains itself even where no
/// [`RequireFeatures`] was declared (the declaration is the better error: it
/// names the missing features before any step runs).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	BypassErrors = BypassErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
	Sequence<Request, Response>,
	ExchangeOverload = sequence_overload(),
)]
pub struct ExchangeSequence;

/// The [`ExchangeOverload`] serving dispatch from a [`Sequence<Request, Response>`]:
/// `Pass` becomes a `200`, `Fail` the failing step's response.
fn sequence_overload() -> ExchangeOverload {
	ActionOverload::new(Action::new_async(
		async |cx: ActionContext<Request>| -> Result<Response> {
			let action = cx
				.caller
				.get(|action: &Action<Request, Outcome<Request, Response>>| {
					action.clone()
				})
				.await?;
			let caller = cx.caller.clone();
			match caller.call_detached(action, cx.input).await {
				Ok(Pass(_)) => Response::ok().xok(),
				Ok(Fail(response)) => response.xok(),
				Err(err) => Err(name_unregistered_children(&caller, err).await),
			}
		},
	))
}

/// Append the names of any [`UnregisteredTag`] descendants to a failed run:
/// the backstop explanation for a binary that loaded steps it never linked,
/// raised at the edge that reports the failure rather than by the sequence
/// itself (which stays blind to how its children loaded).
async fn name_unregistered_children(
	caller: &AsyncEntity,
	err: BevyError,
) -> BevyError {
	caller
		.with_state::<(Query<&UnregisteredTag>, Query<&Children>), _>(
			|entity, (tags, children)| {
				children
					.iter_descendants(entity)
					.filter_map(|child| tags.get(child).ok())
					.map(|tag| format!("`<{}>`", tag.as_str()))
					.collect::<Vec<_>>()
			},
		)
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
	async fn all_passing_children_respond_ok() {
		router_world()
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeSequence,
				children![passing_step(), passing_step()],
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn failing_child_returns_its_response() {
		router_world()
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeSequence,
				children![passing_step(), failing_step(), passing_step()],
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[beet_core::test]
	async fn config_children_are_skipped() {
		router_world()
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeSequence,
				children![Name::new("config-only"), passing_step()],
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	/// A route whose children are all inert — the shape a lean binary loads an
	/// undeclared deploy verb as — fails rather than serving a `200` for work
	/// that never ran, and the edge names the unregistered tags. The sequence
	/// itself stays blind to how its children loaded; the naming happens here.
	#[beet_core::test]
	async fn inert_children_fail_loudly() {
		router_world()
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("deploy"),
				ExchangeSequence,
				children![UnregisteredTag::new("TofuApply")],
			)]))
			.exchange(Request::get("deploy"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("skipped all 1 of its children")
			.xpect_contains("this binary did not register: `<TofuApply>`");
	}

	/// A `() -> Outcome` behavior step serves the sequence through its own
	/// [`ActionOverload`], threading the request onward: the old
	/// `BehaviorSequence` semantics with no registry and no dedicated marker.
	#[beet_core::test]
	async fn overloaded_step_runs() {
		let ran = Store::new(false);
		let recorder = ran.clone();
		router_world()
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("run"),
				ExchangeSequence,
				children![(
					Action::<(), Outcome>::new_pure(move |_: ActionContext| {
						recorder.set(true);
						Outcome::PASS.xok()
					}),
					ActionOverload::<Request, Outcome<Request, Response>>::new(
						Action::new_async(
						async |cx: ActionContext<Request>| -> Result<
							Outcome<Request, Response>,
						> {
							let behavior = cx
								.caller
								.get(|action: &Action<(), Outcome>| action.clone())
								.await?;
							match cx.caller.call_detached(behavior, ()).await? {
								Pass(()) => Outcome::Pass(cx.input),
								Fail(()) => {
									Outcome::Fail(Response::internal_error())
								}
							}
							.xok()
						},
					)),
				)],
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		ran.get().xpect_true();
	}
}
