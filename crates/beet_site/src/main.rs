#![feature(more_qualified_paths)]
use anyhow::Result;



#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
	use beet::prelude::*;
	use beet_site::prelude::*;
	use sweet::prelude::*;

	fn with_sidebar(route: RouteFunc<RsxRouteFunc>) -> RouteFunc<RsxRouteFunc> {
		route.map_func(|func| {
			async move || -> anyhow::Result<WebNode> {
				let root = func().await?;
				Ok(rsx! { <BeetSidebarLayout>{root}</BeetSidebarLayout> })
			}
		})
	}

	DefaultRunner {
		server_actions: beet_site::server_actions::collect(),
		routes: beet_site::pages::collect()
			.xtend(beet::design::mockups::collect().xmap_each(with_sidebar))
			.xtend(beet_site::docs::collect().xmap_each(with_sidebar)),
	}
	.run()?;

	Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<()> {
	beet_site::wasm::collect().mount_with_server_url("https://beetrs.dev")
}
