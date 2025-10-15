use beet_core::prelude::*;
#[allow(unused_imports)]
use beet_rsx::prelude::*;
// use beet_router::types::RouteFunc;
#[allow(unused_imports)]
use crate::prelude::*;

/// Collect all static HTML endpoints in the [`Router`]
pub async fn collect_html(
	world: &mut World,
) -> Result<Vec<(AbsPathBuf, String)>> {
	let workspace_config = world.resource::<WorkspaceConfig>();
	let html_dir = workspace_config.html_dir.into_abs();

	let metas = world.run_system_cached(
		EndpointMeta::collect.pipe(EndpointMeta::static_get_html),
	)?;

	let mut results = Vec::new();
	for meta in metas {
		let path = meta.route_segments().annotated_route_path();
		debug!("building html for {}", &path);

		let route_path = html_dir.join(&path.as_relative()).join("index.html");

		// let route_info = RouteInfo::get(path.clone());

		let text = world
			.oneshot(path.clone())
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
	results.xok()
}
