use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker that makes an entity a sequenced exchange: its `Action<Request, Response>`
/// runs each child action in order, returning [`Response::ok`] if all pass or the
/// first [`Fail`] response (errors convert to a response). A child without a matching
/// action (eg a config-only block) is skipped via the required [`ExcludeErrors`].
///
/// The markup-spreadable form of [`exchange_sequence`]: spread on a routed element so
/// its child actions run as one route. Pair it with [`RoutePath`] for the path, since
/// the [`Route`] template slots its children one level down (under a fragment) but a
/// sequence reads its *direct* children:
///
/// ```bsx
/// <div {(RoutePath("deploy"), ExchangeSequence)}>
///   <MyConfigBlock/>
///   <MyDeployAction/>
/// </div>
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	ExcludeErrors = ExcludeErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
	Action<Request, Response> = exchange_sequence_action(),
)]
pub struct ExchangeSequence;

/// The `Action<Request, Response>` behind [`ExchangeSequence`] and
/// [`exchange_sequence`]: runs the entity's children as a [`SequenceAction`],
/// mapping its outcome to a response.
fn exchange_sequence_action() -> Action<Request, Response> {
	let sequence = SequenceAction::<Request, Response>::default().into_action();
	Action::<Request, Response>::new_async(
		async move |cx: ActionContext<Request>| -> Response {
			match cx.caller.call_detached(sequence, cx.input).await {
				Ok(Pass(_req)) => Response::ok(),
				// child returned Fail — use that response
				Ok(Fail(res)) => res,
				Err(err) => err.into_response(),
			}
		},
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn all_pass() {
		AsyncPlugin::world()
			.spawn((exchange_sequence(), children![
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),
			]))
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn first_fail_stops() {
		AsyncPlugin::world()
			.spawn((exchange_sequence(), children![
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|_cx: ActionContext<Request>| {
						Fail(Response::from_status(StatusCode::IM_A_TEAPOT))
					},
				),
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),
			]))
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[beet_core::test]
	async fn works_with_router() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((default_router(), children![route(
				"seq",
				(exchange_sequence(), children![Action::<
					Request,
					Outcome<Request, Response>,
				>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),]),
			)]))
			.exchange(Request::get("seq"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
