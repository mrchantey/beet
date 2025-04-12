use crate::prelude::*;
use beet_rsx::as_beet::*;

#[derive(Node)]
pub struct Sidebar {
	pub nodes: Vec<SidebarNode>,
}

fn sidebar(Sidebar { nodes }: Sidebar) -> RsxNode {
	rsx! {
		<nav id="sidebar" aria-hidden="false">
		{nodes.into_iter().map(|node|{
			SidebarItem { node, root: true}
		})
		}
		</nav>
		<script src="./sidebar.js"/>
		<style>
			nav{
				--sidebar-width:20rem;
				--sidebar-indent: 0.5rem;
				background-color:var(--bt-color-surface-container-highest);
				padding: 0 0.5.em 0 0;
				width: var(--sidebar-width);
				min-width: var(--sidebar-width);
				max-width: var(--sidebar-width);
				/* overflow-y:scroll; */
			}

		</style>
	}
}
