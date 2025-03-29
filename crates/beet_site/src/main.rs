#![feature(more_qualified_paths)]
use beet::prelude::*;
use beet_site::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let mut routes = beet_site::routes::collect();
	routes.extend(beet::design::mockups::collect().into_iter().map(|route| {
		// wrap the mockups in a beet page
		route.map_func(|func| {
			async move || -> anyhow::Result<RsxRoot> {
				let root = func().await?;
				Ok(rsx! { <BeetPage>{root}</BeetPage> })
			}
		})
	}));
	AppRouter::new(app_cx!()).add_collection(routes).run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
	AppRouter::new(app_cx!())
		.add_collection(beet_site::wasm::collect())
		.run();
}
