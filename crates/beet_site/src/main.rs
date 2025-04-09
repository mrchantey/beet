#![feature(more_qualified_paths)]
#[allow(unused)]
use beet::prelude::*;
#[allow(unused)]
use beet_site::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	// we must collect all or islands break?
	let mut routes = beet_site::routes::collect();
	routes.extend(beet::design::mockups::collect().into_iter().map(|route| {
		// wrap the mockups in a beet page
		route.map_func(|func| {
			async move || -> anyhow::Result<RsxNode> {
				let root = func().await?;
				Ok(rsx! { <BeetSidebarLayout>{root}</BeetSidebarLayout> })
			}
		})
	}));
	routes.extend(beet_site::docs::collect());
	AppRouter::new(app_cx!()).add_collection(routes).run();
}

#[cfg(target_arch = "wasm32")]
fn main() -> anyhow::Result<()> { beet_site::wasm::collect().mount() }
