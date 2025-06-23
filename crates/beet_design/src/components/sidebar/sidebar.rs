use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarInfo {
	#[serde(default)]
	pub label: Option<String>,
	#[serde(default)]
	pub order: Option<u32>,
}


#[template]
pub fn Sidebar(nodes: Vec<SidebarNode>) -> impl Bundle {
	rsx! {
		<nav id="sidebar" aria-hidden="false">
		{nodes.into_iter().map(|node|{
			SidebarItem { node, root: true}
		})
		}
		</nav>
		<script hoist:body src="./sidebar.js"/>
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



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let nodes = vec![SidebarNode {
			display_name: "Home".to_string(),
			path: None,
			children: vec![SidebarNode {
				display_name: "Docs".to_string(),
				path: Some(RoutePath::new("/docs")),
				children: vec![
					SidebarNode {
						display_name: "Testing".to_string(),
						path: Some(RoutePath::new("/docs/testing")),
						children: vec![],
						expanded: false,
					},
					SidebarNode {
						display_name: "Partying".to_string(),
						path: Some(RoutePath::new("/docs/partying")),
						children: vec![],
						expanded: false,
					},
				],
				expanded: false,
			}],
			expanded: true,
		}];

		rsx! {
			<Sidebar nodes=nodes />
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_contain("Partying");
	}
}
