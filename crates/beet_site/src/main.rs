#![feature(more_qualified_paths)]
#[allow(unused)]
use beet::prelude::*;
#[allow(unused)]
use beet_site::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	AppRouter::new(app_cx!())
		.add_collection(beet_site::routes::collect())
		.add_collection(
			beet::design::mockups::collect()
				.into_iter()
				.map(|route| {
					// wrap the mockups in a beet page
					route.map_func(|func| {
						async move || -> anyhow::Result<RsxNode> {
							let root = func().await?;
							Ok(
								rsx! { <BeetSidebarLayout>{root}</BeetSidebarLayout> },
							)
						}
					})
				})
				.collect::<Vec<_>>(),
		)
		.add_collection(beet_site::docs::collect())
		.run();
}

#[cfg(target_arch = "wasm32")]
fn main() -> anyhow::Result<()> {
	beet_site::wasm::collect().mount().unwrap();
	Ok(())
}
