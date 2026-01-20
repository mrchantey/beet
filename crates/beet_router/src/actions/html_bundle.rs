use beet_core::prelude::*;
use beet_dom::prelude::BeetRoot;
use beet_dom::prelude::TemplateOf;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// A [`SystemParam`] for retrieving the [`HtmlBundle`] for
/// a given `exchange`
#[derive(SystemParam)]
pub struct HtmlBundleQuery<'w, 's, F = ()>
where
	F: 'static + QueryFilter,
{
	agent_query: AgentQuery<'w, 's, Entity, With<HtmlBundle>>,
	agents: Query<'w, 's, &'static Children, F>,
	children: Query<'w, 's, &'static ChildOf>,
	html_bundles: Query<'w, 's, Entity, With<HtmlBundle>>,
	templates: Query<'w, 's, &'static TemplateOf>,
}

impl<F> HtmlBundleQuery<'_, '_, F>
where
	F: 'static + QueryFilter,
{
	/// Get the first [`HtmlBundle`] found in the direct children of the agent for the given `action`.
	/// Returns `None` if no [`HtmlBundle`] is found.
	/// ## Errors
	/// if multiple children are found.
	pub fn get(&self, action: Entity) -> Result<Option<Entity>> {
		let agent = self.agent_query.entity(action);

		let Ok(children) = self.agents.get(agent) else {
			return Ok(None);
		};
		let found = children
			.iter()
			.filter_map(|e| self.html_bundles.get(e).ok())
			.collect::<Vec<_>>();

		match found.len() {
			0 => Ok(None),
			1 => Ok(Some(found[0])),
			_ => bevybail!(
				"Multiple HtmlBundle found in exchange children,
				this is usually caused by multiple matching endpoints.
				Please check each has a distinct Method and PathFilter"
			),
		}
	}

	/// Given an entity that is a descendant of an agent, get all actions
	/// associated with that agent. Recursively follows template chains.
	pub fn actions_from_agent_descendant(
		&self,
		entity: Entity,
	) -> Result<&Actions> {
		// Walk up ancestors looking for an entity with Actions component.
		// This handles the case where HtmlBundle is a direct child of the agent
		// (not via ActionOf relationship).
		let mut current = entity;

		loop {
			// Check if current entity has Actions
			if let Ok(actions) = self.agent_query.agents.get(current) {
				return Ok(actions);
			}

			// Try to follow template chain first
			// TemplateOf points to where the template was instantiated,
			// so we continue walking up from there
			if let Ok(template_of) = self.templates.get(current) {
				current = template_of.get();
			} else if let Ok(parent) = self.children.get(current) {
				// Otherwise, walk up the parent chain
				current = parent.get();
			} else {
				// No more ancestors to check
				#[allow(unreachable_code)]
				return bevybail!(
					"Could not find Actions for agent descendant {:?}",
					entity
				);
			}
		}
	}
}


/// An action for converting a [`HtmlBundle`] into a HTML response,
/// only checking direct [`Children`] of the `exchange`.
/// If a response already exists or none are found this action does nothing.
///
/// ## Errors
///
/// Errors if multiple [`HtmlBundle`] are found.
pub fn html_bundle_to_response() -> impl Bundle {
	(
		Name::new("Html Bundle Parser"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 mut commands: Commands,
			 query: HtmlBundleQuery<Without<ResponseMarker>>|
			 -> Result {
				let action = ev.target();
				let agent = query.agent_query.entity(action);
				let Some(html_bundle) = query.get(action)? else {
					commands.entity(action).trigger_target(Outcome::Fail);
					return Ok(());
				};

				commands.queue(move |world: &mut World| -> Result {
					// unless a [`BeetRoot`] is explicitly inserted,
					// we assume this fragment should be wrapped in
					// a [`HtmlDocument`], which also inserts a [`BeetRoot`]
					if !world.entity_mut(html_bundle).contains::<BeetRoot>() {
						world.entity_mut(html_bundle).insert(HtmlDocument);
					}
					world.run_schedule(ApplyDirectives);
					let html = world
						.run_system_cached_with(render_fragment, html_bundle)?;
					world.entity_mut(agent).insert(Html(html).into_response());
					// trigger outcome after response is inserted to ensure
					// outcome_handler sees the ResponseMarker
					world.entity_mut(action).trigger_target(Outcome::Pass);
					Ok(())
				});
				Ok(())
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
	use beet_rsx::prelude::*;

	#[template]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		rsx! {
			<div>foo: {foo}</div>
		}
	}

	#[beet_core::test]
	async fn bundle_to_response_false() {
		RouterPlugin
			.into_world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get().with_action(|| rsx! {"hello world"}),
					// the scene is ignored because we have not inserted
					// html_bundle_to_response. this means the control
					// flow will silently succeed, maybe we should error?
				])
			}))
			.exchange(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn bundle_to_response_true() {
		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get().with_action(|| (
						BeetRoot,
						rsx! {<div>hello world</div>}
					)),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/"))
			.await
			.xpect_eq("<div>hello world</div>");
	}

	#[beet_core::test]
	async fn endpoint_tree_from_agent_descendent() {
		#[template]
		fn Foobar(
			entity: Entity,
			#[field(param)] bundle_query: HtmlBundleQuery,
			#[field(param)] mut route_query: RouteQuery,
		) -> Result<TextNode> {
			let actions =
				bundle_query.actions_from_agent_descendant(entity).unwrap();
			assert_eq!(actions.len(), 1);
			let text = route_query.endpoint_tree(actions[0])?.to_string();
			TextNode::new(text).xok()
		}


		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_path("foo")
						.with_action(|| (BeetRoot, rsx! {<Foobar/>})),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/foo"))
			.await
			.xpect_eq("/foo\n");
	}

	#[beet_core::test]
	async fn nested_template_actions() {
		#[template]
		fn Inner(
			entity: Entity,
			#[field(param)] bundle_query: HtmlBundleQuery,
			#[field(param)] mut route_query: RouteQuery,
		) -> Result<TextNode> {
			let actions =
				bundle_query.actions_from_agent_descendant(entity).unwrap();
			assert_eq!(actions.len(), 1);
			let text = route_query.endpoint_tree(actions[0])?.to_string();
			TextNode::new(text).xok()
		}

		#[template]
		fn Outer() -> impl Bundle {
			rsx! {<Inner/>}
		}

		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_path("nested")
						.with_action(|| (BeetRoot, rsx! {<Outer/>})),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/nested"))
			.await
			.xpect_eq("/nested\n");
	}

	#[beet_core::test]
	async fn deeply_nested_template_actions() {
		#[template]
		fn Level3(
			entity: Entity,
			#[field(param)] bundle_query: HtmlBundleQuery,
			#[field(param)] mut route_query: RouteQuery,
		) -> Result<TextNode> {
			let actions =
				bundle_query.actions_from_agent_descendant(entity).unwrap();
			assert_eq!(actions.len(), 1);
			let text = route_query.endpoint_tree(actions[0])?.to_string();
			TextNode::new(text).xok()
		}

		#[template]
		fn Level2() -> impl Bundle {
			rsx! {<Level3/>}
		}

		#[template]
		fn Level1() -> impl Bundle {
			rsx! {<Level2/>}
		}

		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_path("deep")
						.with_action(|| (BeetRoot, rsx! {<Level1/>})),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/deep"))
			.await
			.xpect_eq("/deep\n");
	}


	#[beet_core::test]
	async fn with_template() {
		RouterPlugin::world()
			// .with_resource(RenderMode::Ssr)
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_action(|| rsx! {<MyTemplate foo=42/>}),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/"))
			.await
			.xpect_eq(
				"<!DOCTYPE html><html><head></head><body><div>foo: 42</div></body></html>",
			);
	}
	#[beet_core::test]
	async fn middleware() {
		RouterPlugin::world()
			// .with_resource(RenderMode::Ssr)
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_action(|| rsx! {<MyTemplate foo=42/>}),
					OnSpawn::observe(
						|ev: On<GetOutcome>,
							agent_query: AgentQuery,
						 query: HtmlBundleQuery<Without<ResponseMarker>>,
						 mut commands: Commands|
						 -> Result {
								let action = ev.target();
							let agent = agent_query.entity(action);
							let Some(html_bundle) = query.get(action)? else {
								commands.entity(action).trigger_target(Outcome::Fail);
								return Ok(());
							};
							commands.spawn((
								HtmlDocument,
								HtmlBundle,
								ChildOf(agent),
								rsx! {
									"middleware!" {html_bundle}
								},
							));
							commands.entity(action).trigger_target(Outcome::Pass);
							Ok(())
						}
					),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/"))
			.await
			.xpect_str("<!DOCTYPE html><html><head></head><body>middleware!<div>foo: 42</div></body></html>");
	}
}
