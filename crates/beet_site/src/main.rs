#![feature(more_qualified_paths)]
use anyhow::Result;



#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
	use beet::exports::axum::Router;
	use beet::prelude::*;
	use beet_site::prelude::*;
	use sweet::prelude::*;


	fn with_sidebar(route: RouteFunc<RsxRouteFunc>) -> RouteFunc<RsxRouteFunc> {
		route.map_func(|func| {
			async move || -> anyhow::Result<RsxNode> {
				let root = func().await?;
				Ok(rsx! { <BeetSidebarLayout>{root}</BeetSidebarLayout> })
			}
		})
	}

	let routes = beet_site::pages::collect()
		.xtend(beet::design::mockups::collect().xmap_each(with_sidebar))
		.xtend(beet_site::docs::collect().xmap_each(with_sidebar));

	let args = AppRouterArgs::parse();

	if args.is_static {
		routes.xpipe(RouteFuncsToHtml::new(args.html_dir)).await?;
	} else {
		let mut router = Router::new();
		for action in server_actions::collect() {
			router = (action.func)(router);
		}

		BeetServer {
			html_dir: args.html_dir.into(),
			router,
			..Default::default()
		}
		.serve()
		.await?;
	}

	Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<()> { beet_site::wasm::collect().mount() }
