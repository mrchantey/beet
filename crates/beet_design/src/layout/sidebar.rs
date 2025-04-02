use beet_router::prelude::*;
use beet_rsx::as_beet::*;
use crate::prelude::*;

#[derive(Node)]
pub struct Sidebar {
	pub routes: StaticRouteTree,
}

fn sidebar(Sidebar { routes }: Sidebar) -> RsxNode {

	rsx! {
		<nav>
		<SidebarGroup tree={routes} root/>
		</nav>
		<style>
			nav{
				display: flex;
				flex-direction: column;
				gap: 1rem;
			}
		</style>
	}
}
