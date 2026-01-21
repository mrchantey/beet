use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;


#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<ServerPlugin>()
			.init_plugin::<ApplyDirectivesPlugin>()
			.init_plugin::<ControlFlowPlugin>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<RenderMode>()
			.init_resource::<HtmlConstants>();
	}
}


#[derive(Debug, Default, Copy, Clone, Resource, PartialEq, Eq)]
pub enum RenderMode {
	/// Static html routes will be skipped, using the [`bucket_handler`] fallback
	/// to serve files from the bucket.
	#[default]
	Ssg,
	/// All static html [`RouteHandler`] funcs will run instead of using the [`bucket_handler`].
	Ssr,
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn works() {
		RouterPlugin::world()
			.spawn(flow_exchange(|| EndWith(Outcome::Pass)))
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
		RouterPlugin::world()
			.spawn(flow_exchange(|| EndWith(Outcome::Fail)))
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
	}

	#[beet_core::test]
	async fn route_tree() {
		let mut world = RouterPlugin::world();
		let func = || {
			(CacheStrategy::Static, children![
				EndpointBuilder::get()
					.with_path("foo")
					.with_cache_strategy(CacheStrategy::Static)
					.with_action(|| "foo"),
				(PathPartial::new("bar"), children![
					EndpointBuilder::get()
						.with_path("bazz")
						.with_cache_strategy(CacheStrategy::Static)
						.with_action(|| "bazz")
				]),
				PathPartial::new("boo"),
			])
		};

		// Test endpoints_from_bundle_func works
		EndpointTree::endpoints_from_bundle_func(&mut world, func.clone())
			.unwrap()
			.iter()
			.map(|p| p.path().annotated_route_path())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				RoutePath::new("/foo"),
				RoutePath::new("/bar/bazz"),
			]);

		// Test that router_exchange spawns EndpointTree on the entity
		let entity = world.spawn(router_exchange(func)).id();
		world
			.entity(entity)
			.get::<EndpointTree>()
			.is_some()
			.xpect_true();
	}

	#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
	#[beet_core::test]
	async fn server() {
		let server = HttpServer::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((server, flow_exchange(|| EndpointBuilder::get())))
				.run();
		});
		time_ext::sleep_millis(10).await;
		Request::get(&url)
			.send()
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::Ok);
	}
}
