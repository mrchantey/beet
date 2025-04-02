mod sidebar_group;
pub use sidebar_group::*;


use crate::prelude::*;
use beet_router::prelude::*;
use beet_rsx::as_beet::*;

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


#[derive(Debug, Default, Clone)]
pub struct StaticRouteTreeToSidebarTree {
	/// All groups that match this filter will be expanded
	pub expanded_filter: GlobFilter,
	/// Include root index file, excluded by default as this
	/// is usually the home page accessed by clicking the logo
	pub include_root_index: bool,
}

impl StaticRouteTreeToSidebarTree {
	fn map_node(&self, tree: StaticRouteTree) -> SidebarNode {
		SidebarNode::Route {
			display_name: "foo".into(),
			path: "bar".into(),
		}
	}
}

impl RsxPipeline<StaticRouteTree, SidebarNode>
	for StaticRouteTreeToSidebarTree
{
	fn apply(self, value: StaticRouteTree) -> SidebarNode {
		self.map_node(value)
	}
}


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
