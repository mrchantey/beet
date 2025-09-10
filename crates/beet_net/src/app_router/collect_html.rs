use beet_core::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;


/// Collect all static HTML endpoints in the [`Router`]
pub async fn collect_html(
	world: &mut World,
) -> Result<Vec<(AbsPathBuf, String)>> {
	let workspace_config = world.resource::<WorkspaceConfig>();
	let html_dir = workspace_config.html_dir.into_abs();

	let router = world.resource::<Router>();

	router
		.construct_world()
		.await
		.run_system_cached(static_get_routes)?
		.into_iter()
		// TODO parallel
		.map(async |(_, path)| -> Result<Option<(AbsPathBuf, String)>> {
			debug!("building html for {}", &path);
			use http::header::CONTENT_TYPE;

			let route_path =
				html_dir.join(&path.as_relative()).join("index.html");

			// let route_info = RouteInfo::get(path.clone());

			let res = router
				.oneshot(path.clone())
				.await
				.into_result()
				.await
				.map_err(|err| {
					bevyhow!("failed to build html for {}\n{}", &path, err)
				})?;

			// we are only collecting html responses, other static endpoints
			// are not exported
			if res.header_contains(CONTENT_TYPE, "text/html") {
				let html = res.text().await?;
				Some((route_path, html))
			} else {
				None
			}
			.xok()
		})
		.xmap(futures::future::try_join_all)
		.await?
		.into_iter()
		.filter_map(|res| res)
		.collect::<Vec<_>>()
		.xok()
}
