#![feature(more_qualified_paths)]
use anyhow::Result;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
	use beet::prelude::*;
	use beet_site::prelude::*;
	use sweet::prelude::*;
	// we must collect all or islands break?

	let routes = beet_site::pages::collect()
		.xtend(beet::design::mockups::collect().xmap_each(|route| {
			// wrap the mockups in a beet page
			route.map_func(|func| {
				async move || -> anyhow::Result<RsxNode> {
					let root = func().await?;
					Ok(rsx! { <BeetSidebarLayout>{root}</BeetSidebarLayout> })
				}
			})
		}))
		.xtend(beet_site::docs::collect());

	let args = AppRouterArgs::parse();

	if args.is_static {
		routes.xpipe(RouteFuncsToHtml::new(args.html_dir)).await?;
	} else {
		BeetServer {
			html_dir: args.html_dir.into(),
			// router: ,
			..Default::default()
		}
		.serve()
		.await?;
	}

	Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<()> { beet_site::wasm::collect().mount() }
