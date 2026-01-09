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
			|ev: On<GetOutcome>,
			 mut commands: Commands,
			 agent_query: AgentQuery<
				(),
				(With<Request>, Without<Response>),
			>| {
				let action = ev.target();
				let outcome = match agent_query.contains(action) {
					true => Outcome::Pass,
					false => Outcome::Fail,
				};
				commands.entity(action).trigger_target(outcome);
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
			|ev: On<GetOutcome>,
			 mut commands: Commands,
			 agent_query: AgentQuery<(), Without<Response>>| {
				let action = ev.target();
				let outcome = match agent_query.contains(action) {
					true => Outcome::Pass,
					false => Outcome::Fail,
				};
				commands.entity(action).trigger_target(outcome);
			},
		),
	)
}

/// Passes only if the `exchange` has a child with a [`HtmlBundle`]
pub fn contains_handler_bundle() -> impl Bundle {
	(
		Name::new("Handler Bundle Predicate"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 mut commands: Commands,
			 children: Query<&Children>,
			 agents: AgentQuery,
			 handler_bundles: Query<(), With<HtmlBundle>>| {
				let action = ev.target();
				let agent = agents.entity(action);
				let outcome = match children
					.iter_direct_descendants(agent)
					.any(|child| handler_bundles.contains(child))
				{
					true => Outcome::Pass,
					false => Outcome::Fail,
				};
				commands.entity(action).trigger_target(outcome);
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
			|ev: On<GetOutcome>,
			 mut commands: Commands,
			 render_mode: Res<RenderMode>| {
				let action = ev.target();
				let outcome = match *render_mode {
					RenderMode::Ssr => Outcome::Pass,
					_ => Outcome::Fail,
				};
				commands.entity(action).trigger_target(outcome);
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

	#[sweet::test]
	async fn fallback_no_response() {
		// request no response
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					common_predicates::fallback(),
					EndpointBuilder::get()
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[sweet::test]
	async fn fallback_request_consumed() {
		// request already consumed
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_handler(StatusCode::IM_A_TEAPOT),
					common_predicates::fallback(),
					EndpointBuilder::get().with_handler(|| -> () {
						unreachable!();
					}),
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[sweet::test]
	async fn is_ssr_true() {
		RouterPlugin::world()
			.xtap(|world| world.insert_resource(RenderMode::Ssr))
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					common_predicates::is_ssr(),
					EndpointBuilder::get()
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[sweet::test]
	async fn is_ssr_false() {
		RouterPlugin::world()
			.xtap(|world| world.insert_resource(RenderMode::Ssg))
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					common_predicates::is_ssr(),
					EndpointBuilder::get(),
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[sweet::test]
	async fn contains_handler_bundle_pass() {
		use beet_rsx::prelude::HtmlBundle;
		// Test that the predicate passes when an upstream action spawns HtmlBundle
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					// First action spawns HtmlBundle as child of agent
					OnSpawn::observe(
						|ev: On<GetOutcome>,
						 agents: AgentQuery,
						 mut commands: Commands| {
							let agent = agents.entity(ev.target());
							commands.entity(agent).with_children(|parent| {
								parent.spawn(HtmlBundle);
							});
							commands
								.entity(ev.target())
								.trigger_target(Outcome::Pass);
						},
					),
					// Then predicate checks for it
					common_predicates::contains_handler_bundle(),
					EndpointBuilder::get()
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[sweet::test]
	async fn contains_handler_bundle_fail() {
		// Test that the predicate fails when no HtmlBundle child exists
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					common_predicates::contains_handler_bundle(),
					EndpointBuilder::get(),
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}
}
