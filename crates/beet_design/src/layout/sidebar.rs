use beet_router::prelude::*;
use beet_rsx::as_beet::*;

#[derive(Node)]
pub struct Sidebar {
	pub routes: StaticRouteTree,
}

fn sidebar(Sidebar { routes }: Sidebar) -> RsxNode {
	let nav_items = routes
		.flatten()
		.iter()
		.map(|route| {
			let route_str = route.to_string_lossy().to_string();
			rsx! { <a href=route_str.clone()>{route_str}</a> }
		})
		.collect::<Vec<_>>();

	rsx! {
		<nav>{nav_items}</nav>
		<style>
			nav{
				display: flex;
				flex-direction: column;
				gap: 1rem;
			}
		</style>
	}
}
