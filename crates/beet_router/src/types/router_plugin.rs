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

	#[sweet::test]
	async fn works() {
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| EndWith(Outcome::Pass)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| EndWith(Outcome::Fail)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
	}

	#[sweet::test]
	async fn route_tree() {
		let mut world = RouterPlugin::world();
		let spawner = ExchangeSpawner::new_flow(|| {
			(CacheStrategy::Static, children![
				EndpointBuilder::get().with_handler(
					async |_: (), action: AsyncEntity| -> Result<String> {
						let tree =
							RouteQuery::with_async(action, |query, entity| {
								query.endpoint_tree(entity)
							})
							.await?;
						tree.to_string().xok()
					}
				),
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
		});

		EndpointTree::endpoints_from_exchange_spawner(&mut world, &spawner)
			.unwrap()
			.iter()
			.map(|p| p.path().annotated_route_path())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				RoutePath::new("/"),
				RoutePath::new("/foo"),
				RoutePath::new("/bar/bazz"),
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
			.xpect_eq(StatusCode::Ok);
	}
}
