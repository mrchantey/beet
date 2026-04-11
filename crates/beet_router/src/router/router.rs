use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Creates a router bundle with help and navigate middleware.
///
/// This is the standard way to set up routing. It includes:
/// - [`Router2`] for route lookup and dispatch
/// - [`HelpHandler`] middleware for `--help` support
/// - [`NavigateHandler`] middleware for `--navigate` support
///
/// Does **not** include a [`SceneToolRenderer`] — that belongs
/// on the server entity since different servers need different
/// rendering strategies.
pub fn router() -> impl Bundle {
	(
		Router2,
		Middleware::<HelpHandler, Request, Response>::default(),
		Middleware::<NavigateHandler, Request, Response>::default(),
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_node::prelude::*;
	use beet_tool::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn route_renders_scene() {
		router_world()
			.spawn((SceneToolRenderer::default(), router(), children![
				scene_func("about", || {
					(Element::new("p"), children![Value::Str(
						"About page".into()
					)])
				}),
			]))
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
			.spawn((SceneToolRenderer::default(), router(), children![
				scene_func("", || {
					(Element::new("p"), children![Value::Str(
						"Root content".into()
					)])
				}),
			]))
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
			.spawn((SceneToolRenderer::default(), router(), children![
				scene_func("", || {
					children![
						(Element::new("h1"), children![Value::Str(
							"My Server".into()
						)]),
						(Element::new("p"), children![Value::Str(
							"welcome!".into()
						)]),
					]
				}),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
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
			.spawn((SceneToolRenderer::default(), router(), children![
				increment(FieldRef::new("count")),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
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
			.spawn((SceneToolRenderer::default(), router(), children![
				increment(FieldRef::new("count")),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
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
			.spawn((SceneToolRenderer::default(), router(), children![
				increment(FieldRef::new("count")),
			]))
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
			.spawn((SceneToolRenderer::default(), router(), children![
				scene_func("", || {
					children![
						(Element::new("h1"), children![Value::Str(
							"My Server".into()
						)]),
						(Element::new("p"), children![Value::Str(
							"welcome!".into()
						)]),
					]
				}),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
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
			.spawn((SceneToolRenderer::default(), router(), children![
				(
					scene_func("counter", || {
						(Element::new("p"), children![Value::Str(
							"counter".into()
						)])
					}),
					children![increment(FieldRef::new("count")),],
				),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
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
			.spawn((SceneToolRenderer::default(), router(), children![
				increment(FieldRef::new("count")),
			]))
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
			.spawn((SceneToolRenderer::default(), router(), children![
				(
					scene_func("counter", || {
						(Element::new("p"), children![Value::Str(
							"counter".into()
						)])
					}),
					children![increment(FieldRef::new("count")),],
				),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
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
