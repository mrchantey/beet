use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// A more strict version of [`no_response`],
/// passes only if the `exchange` has a [`Request`] but no [`Response`].
/// The no request check indicates the [`Request`] was not consumed by a handler
/// and replaced by a partial response pattern like [`HtmlBundle`].
/// Useful for fallback endpoints like a 404 page.
pub fn fallback() -> impl Bundle {
	(
		Name::new("Fallback Predicate"),
		OnSpawn::observe(
			|mut ev: On<GetOutcome>,
			 exchange: Query<(), (With<Request>, Without<Response>)>| {
				match exchange.contains(ev.agent()) {
					true => ev.trigger_with_cx(Outcome::Pass),
					false => ev.trigger_with_cx(Outcome::Fail),
				};
			},
		),
	)
}

/// Passes only if the `exchange` has no [`Response`],
/// disregarding whether there is a [`Request`] or not.
pub fn no_response() -> impl Bundle {
	(
		Name::new("No Response Predicate"),
		OnSpawn::observe(
			|mut ev: On<GetOutcome>, exchange: Query<(), Without<Response>>| {
				match exchange.contains(ev.agent()) {
					true => ev.trigger_with_cx(Outcome::Pass),
					false => ev.trigger_with_cx(Outcome::Fail),
				};
			},
		),
	)
}

/// Passes only if the `exchange` has a child with a [`HtmlBundle`]
pub fn contains_handler_bundle() -> impl Bundle {
	(
		Name::new("Handler Bundle Predicate"),
		OnSpawn::observe(
			|mut ev: On<GetOutcome>,
			 children: Query<&Children>,
			 handler_bundles: Query<(), With<HtmlBundle>>| {
				match children
					.iter_direct_descendants(ev.agent())
					.any(|child| handler_bundles.contains(child))
				{
					true => ev.trigger_with_cx(Outcome::Pass),
					false => ev.trigger_with_cx(Outcome::Fail),
				};
			},
		),
	)
}

/// Passes only with the [`RenderMode::Ssr`] resource.
///
/// ## Panics
/// Panics if there is no [`RenderMode`] resource.
pub fn is_ssr() -> impl Bundle {
	(
		Name::new("SSR Predicate"),
		OnSpawn::observe(
			|mut ev: On<GetOutcome>, render_mode: Res<RenderMode>| {
				match *render_mode == RenderMode::Ssr {
					true => ev.trigger_with_cx(Outcome::Pass),
					false => ev.trigger_with_cx(Outcome::Fail),
				};
			},
		),
	)
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
		RouterPlugin::world()
			.spawn((Router, Sequence, children![
				common_predicates::fallback(),
				EndpointBuilder::get()
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		// request already consumed
		RouterPlugin::world()
			.spawn((Router, Sequence, children![
				EndpointBuilder::get().with_handler(StatusCode::IM_A_TEAPOT),
				common_predicates::fallback(),
				EndpointBuilder::get(),
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[sweet::test]
	async fn is_ssr() {
		RouterPlugin::world()
			.xtap(|world| world.insert_resource(RenderMode::Ssr))
			.spawn((Router, Sequence, children![
				common_predicates::is_ssr(),
				EndpointBuilder::get()
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		RouterPlugin::world()
			.xtap(|world| world.insert_resource(RenderMode::Ssg))
			.spawn((Router, Sequence, children![
				common_predicates::is_ssr(),
				EndpointBuilder::get(),
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[sweet::test]
	async fn contains_handler_bundle() {
		use beet_rsx::prelude::HtmlBundle;
		// request no response

		RouterPlugin::world()
			.spawn((Router, Sequence, children![
				common_predicates::contains_handler_bundle(),
				EndpointBuilder::get()
			]))
			.oneshot_bundle((Request::get("/"), children![HtmlBundle]))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		// request already consumed
		RouterPlugin::world()
			.spawn((Router, Sequence, children![
				common_predicates::contains_handler_bundle(),
				EndpointBuilder::get(),
			]))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}
}
