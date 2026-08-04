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
/// [`ExcludeErrors`]; a step that is natively another shape carries its own
/// [`ActionOverload`].
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	ExcludeErrors = ExcludeErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
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
			match cx.caller.call_detached(action, cx.input).await? {
				Pass(_) => Response::ok(),
				Fail(response) => response,
			}
			.xok()
		},
	))
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
			.spawn((default_router(), children![(
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
			.spawn((default_router(), children![(
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
			.spawn((default_router(), children![(
				PathPartial::new("run"),
				ExchangeSequence,
				children![Name::new("config-only"), passing_step()],
			)]))
			.exchange(Request::get("run"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	/// A `() -> Outcome` behavior step serves the sequence through its own
	/// [`ActionOverload`], threading the request onward: the old
	/// `BehaviorSequence` semantics with no registry and no dedicated marker.
	#[beet_core::test]
	async fn overloaded_step_runs() {
		let ran = Store::new(false);
		let recorder = ran.clone();
		router_world()
			.spawn((default_router(), children![(
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
