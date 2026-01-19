use beet_core::prelude::*;
use beet_net::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;

/// Collect all static HTML endpoints in the [`Router`]
pub async fn collect_html(
	world: &AsyncWorld,
	exchange_spawner: &ExchangeSpawner,
) -> Result<Vec<(AbsPathBuf, String)>> {
	let html_dir = world
		.with_resource_then::<WorkspaceConfig, _>(|conf| {
			conf.html_dir.into_abs()
		})
		.await;

	let exchange_spawner2 = exchange_spawner.clone();
	// Spawn trees from ExchangeSpawners and collect their endpoints
	let endpoints: Vec<Endpoint> = world
		.with_then(move |world| {
			EndpointTree::endpoints_from_exchange_spawner(
				world,
				&exchange_spawner2,
			)?
			.into_iter()
			// Filter for static GET/HTML endpoints
			.filter(|endpoint| endpoint.is_static_get_html())
			.collect::<Vec<_>>()
			.xok::<BevyError>()
		})
		.await?;

	debug!("building {} static html documents", endpoints.len());

	let mut results = Vec::new();
	// Spawn the exchange spawner to handle oneshot requests
	let spawner_entity = world.spawn_then(exchange_spawner.clone()).await;

	for endpoint in endpoints {
		let path = endpoint.path().annotated_route_path();
		trace!("building html for {}", &path);

		let route_path = html_dir.join(&path.as_relative()).join("index.html");

		let text = spawner_entity
			.exchange(Request::get(&path))
			.await
			.into_result()
			.await
			.map_err(|err| {
				bevyhow!("failed to build html for {}\n{}", &path, err)
			})?
			.text()
			.await?;
		results.push((route_path, text));
	}

	spawner_entity.despawn().await;

	debug!("collected {} static html documents", results.len());
	results.xok()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn children() {
		let mut world = RouterPlugin::world();
		let spawner = flow_exchange(|| {
			(InfallibleSequence, children![
				EndpointBuilder::get()
					.with_path("foo")
					.with_handler(|| "foo")
					.with_cache_strategy(CacheStrategy::Static)
					.with_content_type(ContentType::Html),
				EndpointBuilder::get()
					.with_path("bar")
					.with_handler(|| "bar")
					.with_cache_strategy(CacheStrategy::Static)
					.with_content_type(ContentType::Html),
				// non-static
				EndpointBuilder::get()
					.with_path("bazz")
					.with_handler(|| "bazz")
					.with_content_type(ContentType::Html),
				// non-html
				EndpointBuilder::get()
					.with_path("boo")
					.with_handler(|| "boo")
					.with_cache_strategy(CacheStrategy::Static),
			])
		});

		// actually spawn it for the oneshots
		world.spawn(spawner.clone());

		let ws_path = WorkspaceConfig::default().html_dir.into_abs();
		world
			.run_async_then(async move |world| {
				collect_html(&world, &spawner).await
			})
			.await
			.unwrap()
			.xpect_eq(vec![
				(ws_path.join("foo/index.html"), "foo".to_string()),
				(ws_path.join("bar/index.html"), "bar".to_string()),
			]);
	}
}
