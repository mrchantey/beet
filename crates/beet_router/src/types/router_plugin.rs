use crate::prelude::*;
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
			.init_resource::<HtmlConstants>()
			.add_systems(PostStartup, insert_route_tree);

		// #[cfg(all(
		// 	not(target_arch = "wasm32"),
		// 	not(test),
		// 	feature = "server"
		// ))]
	}
}

/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	match EndpointTree::from_world(world) {
		Ok(tree) => {
			world.insert_resource(tree);
		}
		Err(err) => {
			error!("Failed to build EndpointTree: {}", err);
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[sweet::test]
	async fn works() {
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| EndWith(Outcome::Pass)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| EndWith(Outcome::Fail)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[test]
	fn route_tree() {
		let mut world = World::new();
		world.spawn(ExchangeSpawner::new_flow(|| {
			(CacheStrategy::Static, children![
				EndpointBuilder::get()
					.with_handler(|tree: Res<EndpointTree>| tree.to_string()),
				(EndpointBuilder::get()
					.with_path("foo")
					.with_cache_strategy(CacheStrategy::Static)
					.with_handler(|| "foo")),
				(PathPartial::new("bar"), children![
					EndpointBuilder::get()
						.with_path("bazz")
						.with_cache_strategy(CacheStrategy::Static)
						.with_handler(|| "bazz")
				]),
				PathPartial::new("boo"),
			])
		}));
		world.run_system_cached(insert_route_tree).unwrap();
		world
			.remove_resource::<EndpointTree>()
			.unwrap()
			.flatten()
			.iter()
			.map(|p| p.annotated_route_path())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				RoutePath::new("/"),
				RoutePath::new("/bar/bazz"),
				RoutePath::new("/foo"),
			]);
	}

	#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
	#[sweet::test]
	async fn server() {
		let server = HttpServer::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((
					server,
					ExchangeSpawner::new_flow(|| EndpointBuilder::get()),
				))
				.run();
		});
		time_ext::sleep_millis(10).await;
		Request::get(&url)
			.send()
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
