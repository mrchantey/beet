use crate::prelude::*;

#[template]
pub fn Sidebar(nodes: Vec<SidebarNode>) -> impl Bundle {
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
				--sidebar-width:15rem;
				--sidebar-indent: 0.5rem;
				background-color:var(--bt-color-surface-container-low);
				padding: 0.5 0.5.em 0 0;
				width: var(--sidebar-width);
				min-width: var(--sidebar-width);
				max-width: var(--sidebar-width);
				/* overflow-y:scroll; */
			}

		</style>
	}
}
