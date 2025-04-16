mod sidebar;
mod sidebar_item;
use beet_rsx::prelude::*;
use beet_router::prelude::*;
use heck::ToTitleCase;
pub use sidebar::*;
pub use sidebar_item::*;
use sweet::prelude::GlobFilter;


#[derive(Debug, Default, Clone)]
pub struct RoutePathTreeToSidebarTree {
	/// All groups that match this filter will be expanded
	pub expanded_filter: GlobFilter,
	/// By default the root is unwrapped, enable this to return the root node
	pub keep_root: bool,
}

impl RoutePathTreeToSidebarTree {
	fn map_node(&self, tree: RoutePathTree) -> SidebarNode {
		SidebarNode {
			display_name: tree.name.as_str().to_title_case(),
			expanded: tree
				.path
				.as_ref()
				.map_or(false, |path| self.expanded_filter.passes(&path)),
			path: tree.path,
			children: tree
				.children
				.into_iter()
				.map(|child| self.map_node(child))
				.collect::<Vec<_>>(),
		}
	}
}

impl Pipeline<RoutePathTree, Vec<SidebarNode>> for RoutePathTreeToSidebarTree {
	fn apply(self, value: RoutePathTree) -> Vec<SidebarNode> {
		let root_node = self.map_node(value);
		if self.keep_root {
			vec![root_node]
		} else {
			root_node.children
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarNode {
	/// A Title Case name for the group
	display_name: String,
	/// if this node has a route, this is its path
	path: Option<RoutePath>,
	/// all paths available at this level of the tree
	children: Vec<SidebarNode>,
	/// expanded portions of the tree
	expanded: bool,
}
