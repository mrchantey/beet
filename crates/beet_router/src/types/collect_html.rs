use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;

/// Collect all static HTML endpoints in the [`Router`]
pub async fn collect_html(
	world: &AsyncWorld,
	func: impl BundleFunc,
) -> Result<Vec<(AbsPathBuf, String)>> {
	let html_dir = world
		.with_resource_then::<WorkspaceConfig, _>(|conf| {
			conf.html_dir.into_abs()
		})
		.await;

	let func2 = func.clone();
	// Spawn trees from ExchangeSpawners and collect their endpoints
	let endpoints: Vec<Endpoint> = world
		.with_then(move |world| {
			EndpointTree::endpoints_from_bundle_func(world, func2)?
				.into_iter()
				// Filter for static GET/HTML endpoints
				.filter(|endpoint| endpoint.is_static_get_html())
				.collect::<Vec<_>>()
				.xok::<BevyError>()
		})
		.await?;

	debug!("building {} static html documents", endpoints.len());

	let mut results = Vec::new();
	// Spawn the exchange spawner to handle oneshot requests.
	// Wrap endpoints with html_bundle_to_response() so RSX bundles
	// are converted to Response before the exchange completes.
	let server_entity = world
		.spawn_then(flow_exchange(move || {
			(InfallibleSequence, children![
				func.clone().bundle_func(),
				html_bundle_to_response(),
			])
		}))
		.await;

	for endpoint in endpoints {
		let path = endpoint.path().annotated_route_path();
		trace!("building html for {}", &path);

		let route_path = html_dir.join(&path.as_relative()).join("index.html");

		let text = server_entity
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

	server_entity.despawn().await;

	debug!("collected {} static html documents", results.len());
	results.xok()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::BeetRoot;
	use beet_flow::prelude::*;
	use beet_rsx::prelude::*;

	#[beet_core::test]
	async fn children() {
		let mut world = RouterPlugin::world();
		let func = || {
			(InfallibleSequence, children![
				EndpointBuilder::get()
					.with_path("foo")
					.with_action(|| "foo")
					.with_cache_strategy(CacheStrategy::Static)
					.with_response_body(BodyType::html()),
				EndpointBuilder::get()
					.with_path("bar")
					.with_action(|| "bar")
					.with_cache_strategy(CacheStrategy::Static)
					.with_response_body(BodyType::html()),
				// non-static
				EndpointBuilder::get()
					.with_path("bazz")
					.with_action(|| "bazz")
					.with_response_body(BodyType::html()),
				// non-html
				EndpointBuilder::get()
					.with_path("boo")
					.with_action(|| "boo")
					.with_cache_strategy(CacheStrategy::Static),
			])
		};

		let ws_path = WorkspaceConfig::default().html_dir.into_abs();
		world
			.run_async_then(async move |world| collect_html(&world, func).await)
			.await
			.unwrap()
			.xpect_eq(vec![
				(ws_path.join("foo/index.html"), "foo".to_string()),
				(ws_path.join("bar/index.html"), "bar".to_string()),
			]);
	}

	/// Test that RSX endpoints returning HtmlBundle are properly
	/// converted to HTML via html_bundle_to_response
	#[beet_core::test]
	async fn rsx_endpoints() {
		let mut world = RouterPlugin::world();
		let func = || {
			(InfallibleSequence, children![
				EndpointBuilder::get()
					.with_path("rsx-page")
					.with_action(|| (BeetRoot, rsx! {<div>hello rsx</div>}))
					.with_cache_strategy(CacheStrategy::Static)
					.with_response_body(BodyType::html()),
			])
		};

		let ws_path = WorkspaceConfig::default().html_dir.into_abs();
		world
			.run_async_then(async move |world| collect_html(&world, func).await)
			.await
			.unwrap()
			.xpect_eq(vec![(
				ws_path.join("rsx-page/index.html"),
				"<div>hello rsx</div>".to_string(),
			)]);
	}
}
