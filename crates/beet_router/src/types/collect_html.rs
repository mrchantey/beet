use beet_core::prelude::*;
use beet_net::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;

/// Collect all static HTML endpoints in the [`Router`]
pub async fn collect_html(
	world: AsyncWorld,
) -> Result<Vec<(AbsPathBuf, String)>> {
	let html_dir = world
		.with_resource_then::<WorkspaceConfig, _>(|conf| {
			conf.html_dir.into_abs()
		})
		.await;

	let metas = world
		.run_system_cached(
			EndpointMeta::collect.pipe(EndpointMeta::static_get_html),
		)
		.await?;

	debug!("building {} static html documents", metas.len());

	let mut results = Vec::new();

	for meta in metas {
		let path = meta.route_segments().annotated_route_path();
		trace!("building html for {}", &path);

		let route_path = html_dir.join(&path.as_relative()).join("index.html");

		// let route_info = RouteInfo::get(path.clone());

		let text = world
			.oneshot(Request::get(&path))
			.await?
			// .with_then(|world| world.oneshot(path.clone()))
			// .await
			.into_result()
			.await
			.map_err(|err| {
				bevyhow!("failed to build html for {}\n{}", &path, err)
			})?
			.text()
			.await?;
		results.push((route_path, text));
	}
	debug!("collected {} static html documents", results.len());
	results.xok()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn children() {
		let mut world = RouterPlugin::world();
		world.spawn((Router, InfallibleSequence, children![
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
		]));
		let ws_path = WorkspaceConfig::default().html_dir.into_abs();
		world
			.run_async_then(collect_html)
			.await
			.unwrap()
			.xpect_eq(vec![
				(ws_path.join("foo/index.html"), "foo".to_string()),
				(ws_path.join("bar/index.html"), "bar".to_string()),
			]);
	}
}
