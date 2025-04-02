mod sidebar_item;
pub use sidebar_item::*;


use beet_router::prelude::*;
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
				background-color:var(--bt-color-surface-container-highest);
				padding: 0 0.5.em 0 0;
				width: var(--sidebar-width);
				min-width: var(--sidebar-width);
				max-width: var(--sidebar-width);
				/* overflow-y:scroll; */
			}
			nav[aria-hidden="false"]{
				display:block;
			}
			nav[aria-hidden="true"]{
				display:none;
			}
		</style>
	}
}


#[derive(Debug, Default, Clone)]
pub struct StaticRouteTreeToSidebarTree {
	/// All groups that match this filter will be expanded
	pub expanded_filter: GlobFilter,
	/// By default the root is unwrapped, enable this to return the root node
	pub keep_root: bool,
}

impl StaticRouteTreeToSidebarTree {
	fn map_node(&self, tree: StaticRouteTree) -> SidebarNode {
		let display_name = heck::ToTitleCase::to_title_case(tree.name.as_str());

		let path = if let Some(index) = tree
			.paths
			.iter()
			.find(|p| p.file_stem().map(|s| s == "index").unwrap_or(false))
		{
			Some(index.clone())
		} else {
			None
		};

		let mut children = tree
			.children
			.into_iter()
			.map(|child| self.map_node(child))
			.collect::<Vec<_>>();
		children.extend(
			tree.paths
				.into_iter()
				.filter(|p| {
					p.file_stem().map(|s| s != "index").unwrap_or(false)
				})
				.map(|path| SidebarNode::Route {
					display_name: heck::ToTitleCase::to_title_case(
						path.file_stem()
							.map(|s| s.to_str())
							.flatten()
							.unwrap_or("unknown"),
					),
					path,
				})
				.collect::<Vec<_>>(),
		);

		SidebarNode::Group {
			display_name,
			path,
			children,
			expanded: false,
		}
	}
}

impl RsxPipeline<StaticRouteTree, Vec<SidebarNode>>
	for StaticRouteTreeToSidebarTree
{
	fn apply(self, value: StaticRouteTree) -> Vec<SidebarNode> {
		let root_node = self.map_node(value);
		if self.keep_root {
			vec![root_node]
		} else {
			match root_node {
				SidebarNode::Group { children, .. } => children,
				SidebarNode::Route { display_name, path } => {
					vec![SidebarNode::Route { display_name, path }]
				}
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarNode {
	/// a group of routes
	Group {
		/// A Title Case name for the group
		display_name: String,
		/// if this group has an index file, this is its path
		path: Option<RoutePath>,
		/// all paths available at this level of the tree
		children: Vec<SidebarNode>,
		/// expanded portions of the tree
		expanded: bool,
	},
	/// a single route
	Route {
		/// A Title Case name for the route
		display_name: String,
		path: RoutePath,
	},
}
