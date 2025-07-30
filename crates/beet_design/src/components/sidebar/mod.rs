mod sidebar;
mod sidebar_item;
use crate::types::ArticleMeta;
use beet_core::prelude::*;
use beet_rsx::as_beet::GlobFilter;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use heck::ToTitleCase;
pub use sidebar::*;
pub use sidebar_item::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarNode {
	/// A Title Case name for the group
	pub display_name: String,
	/// if this node has a route, this is its path
	pub path: Option<RoutePath>,
	/// all paths available at this level of the tree
	pub children: Vec<SidebarNode>,
	/// expanded portions of the tree
	pub expanded: bool,
}


impl SidebarNode {
	pub fn collect(world: &mut World, expanded_filter: &GlobFilter) -> Self {
		let nodes = ResolvedEndpoint::collect_static_get(world);
		let info_map = nodes
			.iter()
			.map(|(entity, endpoint)| {
				(endpoint.path(), world.entity(*entity).get::<ArticleMeta>())
			})
			.collect::<HashMap<_, _>>();

		let path_tree = RoutePathTree::from_paths(
			nodes
				.iter()
				.map(|(_, endpoint)| endpoint.path())
				.cloned()
				.collect(),
		);
		Self::map_node(&expanded_filter, &info_map, path_tree)
	}

	fn map_node(
		expanded_filter: &GlobFilter,
		info_map: &HashMap<&RoutePath, Option<&ArticleMeta>>,
		node: RoutePathTree,
	) -> Self {
		let meta = info_map.get(&node.route).and_then(|meta| *meta);

		let children = node
			.children
			.iter()
			.map(|child| {
				Self::map_node(expanded_filter, info_map, child.clone())
			})
			.collect();

		// Helper to get a display name from a RoutePath
		fn route_name(route: &RoutePath) -> String {
			let s = route.to_string();
			if s == "/" {
				"Root".to_string()
			} else {
				s.trim_start_matches('/').to_string()
			}
		}

		Self {
			display_name: meta
				.map(|m| m.sidebar.label.clone())
				.flatten()
				.unwrap_or_else(|| route_name(&node.route).to_title_case()),
			path: if node.exists {
				Some(node.route.clone())
			} else {
				None
			},
			children,
			expanded: expanded_filter.passes(&node.route.0),
		}
	}
}
