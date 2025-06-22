use beet::prelude::*;
use beet_site::prelude::*;


#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result {
	AppRouter::default()
		.add_plugins((PagesPlugin, DocsPlugin, ActionsPlugin))
		.run()
	// fn with_sidebar(route: RouteFunc<RsxRouteFunc>) -> RouteFunc<RsxRouteFunc> {
	// 	route.map_func(|func| {
	// 		async move || -> anyhow::Result<WebNode> {
	// 			let root = func().await?;
	// 			Ok(rsx! { <BeetSidebarLayout>{root}</BeetSidebarLayout> })
	// 		}
	// 	})
	// }
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<()> {
	beet_site::wasm::collect().mount_with_server_url("https://beetrs.dev")
}
