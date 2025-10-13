use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// Used on fallback handlers, ie 404 page.
/// Passes only if the `exchange` has a [`Request`] but no [`Response`].
/// The no request check indicates the [`Request`] was not consumed by a handler
/// and replaced by a partial response pattern like [`HandlerBundle`].
pub fn fallback() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>,
		 exchange: Query<(), (With<Request>, Without<Response>)>| {
			match exchange.contains(ev.agent()) {
				true => ev.trigger_next(Outcome::Pass),
				false => ev.trigger_next(Outcome::Fail),
			};
		},
	)
}

/// Passes only if the `exchange` has no [`Response`],
/// disregarding whether there is a [`Request`] or not.
pub fn no_response() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>, exchange: Query<(), Without<Response>>| {
			match exchange.contains(ev.agent()) {
				true => ev.trigger_next(Outcome::Pass),
				false => ev.trigger_next(Outcome::Fail),
			};
		},
	)
}

/// Passes only if the `exchange` has a child with a [`HandlerBundle`]
pub fn contains_handler_bundle() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>,
		 children: Query<&Children>,
		 handler_bundles: Query<(), With<HandlerBundle>>| {
			match children
				.iter_direct_descendants(ev.agent())
				.any(|child| handler_bundles.contains(child))
			{
				true => ev.trigger_next(Outcome::Pass),
				false => ev.trigger_next(Outcome::Fail),
			};
		},
	)
}

/// Passes only with the [`RenderMode::Ssr`] resource.
///
/// ## Panics
/// Panics if there is no [`RenderMode`] resource.
pub fn is_ssr() -> impl Bundle {
	OnSpawn::observe(|mut ev: On<GetOutcome>, render_mode: Res<RenderMode>| {
		match *render_mode == RenderMode::Ssr {
			true => ev.trigger_next(Outcome::Pass),
			false => ev.trigger_next(Outcome::Fail),
		};
	})
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn fallback() {
		// request no response
		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![
				common_predicates::fallback(),
				Endpoint::get()
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		// request already consumed
		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![
				Endpoint::get().with_handler(StatusCode::IM_A_TEAPOT),
				common_predicates::fallback(),
				Endpoint::get(),
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[sweet::test]
	async fn is_ssr() {
		FlowRouterPlugin::world()
			.xtap(|world| world.insert_resource(RenderMode::Ssr))
			.spawn((RouteServer, Sequence, children![
				common_predicates::is_ssr(),
				Endpoint::get()
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		FlowRouterPlugin::world()
			.xtap(|world| world.insert_resource(RenderMode::Ssg))
			.spawn((RouteServer, Sequence, children![
				common_predicates::is_ssr(),
				Endpoint::get(),
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[sweet::test]
	async fn contains_handler_bundle() {
		use beet_rsx::prelude::HandlerBundle;
		// request no response

		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![
				common_predicates::contains_handler_bundle(),
				Endpoint::get()
			]))
			.oneshot_bundle((Request::get("/"), children![HandlerBundle]))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		// request already consumed
		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![
				common_predicates::contains_handler_bundle(),
				Endpoint::get(),
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}
}
