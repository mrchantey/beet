use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Creates a router bundle with logging, help, and navigate middleware.
///
/// This is the standard way to set up routing. It includes:
/// - [`Router`] for route lookup and dispatch
/// - [`RequestLogger`] middleware for per-request logging
/// - [`HelpHandler`] middleware for `--help` support
/// - [`NavigateHandler`] middleware for `--navigate` support
///
/// Does **not** include a [`SceneActionRenderer`] — rendering falls
/// back to the default renderer when no [`SceneActionRenderer`] is
/// found on an ancestor.
pub fn router() -> impl Bundle {
	(
		Router,
		Middleware::<RequestLogger, Request, Response>::default(),
		Middleware::<HelpHandler, Request, Response>::default(),
		Middleware::<NavigateHandler, Request, Response>::default(),
	)
}

/// Routes a request to the matching action in the [`RouteTree`],
/// applying ancestor [`MiddlewareList`] around the matched action.
///
/// When no route matches, renders contextual not-found help.
/// Middleware such as [`HelpHandler`] and [`NavigateHandler`] wrap
/// the inner action so they can intercept before dispatch.
#[action]
#[derive(Debug, Clone, Component)]
pub async fn Router(cx: ActionContext<Request>) -> Response {
	let caller = cx.caller.clone();
	let world = cx.world();
	let request = cx.input;
	let path = request.path().clone();

	// find the matching route in the tree
	let node = world
		.with_state::<AncestorQuery<&RouteTree>, _>(move |query| {
			query.get(caller.id()).map(|tree| tree.find(&path).cloned()).map_err(|_|{
				bevyhow!("Route tree not found. Was the `ActionMeta` added? was the `RouterPlugin` added?")
			})
		})
		.await;

	// resolve the inner action and dispatch entity from the matched route
	let (inner_action, dispatch_entity) = match &node {
		Ok(Some(node)) => {
			let entity = world.entity(node.entity);
			match entity.clone().get_cloned::<ExchangeAction>().await {
				Ok(action) => (action.into_action(), entity),
				Err(err) => return err.into_response(),
			}
		}
		Ok(None) => {
			// no matching route — build a not-found response through
			// the contextual help system so middleware still applies
			(ContextualNotFound.into_action(), cx.caller.clone())
		}
		Err(err) => return bevyhow!("{err}").into_response(),
	};

	dispatch_entity
		.call_with_middleware(inner_action, request)
		.await
		.unwrap_or_else(|err| err.into_response())
}


/// A route that returns a [`SceneEntity`] is a scene route.
/// The entity is rendered via the ancestor [`SceneActionRenderer`]
/// and then converted to a response. Ephemeral scene entities
/// are cleaned up after rendering.
impl ExchangeRouteOut<Self> for SceneEntity {
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
			SceneActionRenderer::render_entity(&caller, self, parts).await
		})
	}
}

/// A route output representing a scene entity to be rendered.
/// Entities in `despawn` are cleaned up after rendering,
/// ie help pages, not-found pages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneEntity {
	/// The entity to render.
	pub entity: Entity,
	/// Entities to despawn after rendering.
	pub despawn: Vec<Entity>,
}
impl SceneEntity {
	/// A scene entity that should not be despawned after render.
	pub fn new_fixed(entity: Entity) -> Self {
		Self {
			entity,
			despawn: default(),
		}
	}

	/// A scene entity that should be despawned after render,
	/// ie a help page or not found route.
	pub fn new_ephemeral(entity: Entity) -> Self {
		Self {
			entity,
			despawn: vec![entity],
		}
	}
	pub fn push_despawn(mut self, entity: Entity) -> Self {
		self.despawn.push(entity);
		self
	}

	/// Merge another scene's despawn list into this one.
	pub fn with_join(mut self, child: SceneEntity) -> Self {
		self.despawn.extend(child.despawn);
		self
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_node::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn route_renders_scene() {
		router_world()
			.spawn((router(), children![fixed_scene(
				"about",
				rsx! { <p>"About page"</p> }
			),]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.contains("About page")
			.xpect_true();
	}

	#[beet_core::test]
	async fn route_renders_root_scene_on_empty_path() {
		router_world()
			.spawn((router(), children![fixed_scene(
				"",
				rsx! { <p>"Root content"</p> }
			),]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn route_renders_root_scene_child() {
		let body = router_world()
			.spawn((router(), children![
				fixed_scene(
					"",
					rsx! { <h1>"My Server"</h1> <p>"welcome!"</p> }
				),
				fixed_scene("about", rsx! { <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("My Server").xpect_true();
		body.contains("welcome!").xpect_true();
	}

	#[beet_core::test]
	async fn help_flag_returns_route_list() {
		router_world()
			.spawn((router(), children![
				increment(FieldRef::new("count")),
				fixed_scene("about", rsx! { <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn dispatches_help_request() {
		router_world()
			.spawn((router(), children![
				increment(FieldRef::new("count")),
				fixed_scene("about", rsx! { <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn not_found() {
		router_world()
			.spawn((router(), children![increment(FieldRef::new("count")),]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn renders_root_scene_on_empty_args() {
		router_world()
			.spawn((router(), children![
				fixed_scene(
					"",
					rsx! { <h1>"My Server"</h1> <p>"welcome!"</p> }
				),
				fixed_scene("about", rsx! { <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::from_cli_str("").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("My Server")
			.xpect_contains("welcome!");
	}

	#[beet_core::test]
	async fn scoped_help_for_subcommand() {
		let mut world = router_world();

		let root = world
			.spawn((router(), children![
				(
					fixed_scene(
						"counter",
						Element::new("p").with_inner_text("counter")
					),
					children![increment(FieldRef::new("count")),],
				),
				fixed_scene("about", rsx! { <p>"about"</p> }),
			]))
			.flush();

		let res = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("counter --help").unwrap(),
			)
			.await
			.unwrap();

		let body = res.unwrap_str().await;
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}

	#[beet_core::test]
	async fn not_found_shows_ancestor_help() {
		router_world()
			.spawn((router(), children![increment(FieldRef::new("count")),]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn not_found_shows_scoped_ancestor_help() {
		router_world()
			.spawn((router(), children![
				(
					fixed_scene(
						"counter",
						Element::new("p").with_inner_text("counter")
					),
					children![increment(FieldRef::new("count")),],
				),
				fixed_scene("about", rsx! { <p>"about"</p> }),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("counter nonsense").unwrap(),
			)
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("increment")
			.xnot()
			.xpect_contains("about");
	}
}
