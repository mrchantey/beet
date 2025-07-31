use crate::prelude::*;
use crate::types::ArticleMeta;
use beet_core::prelude::*;
use beet_rsx::as_beet::GlobFilter;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use heck::ToTitleCase;
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
		{nodes.into_iter().map(|node|
			rsx!{<SidebarItem root node=node/>})
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


pub struct CollectSidebarNode{
	pub include_filter: GlobFilter,
	/// Set some paths to expanded by default,
	/// useful for directories without an index that dont
	/// have an `[ArticleMeta]`.
	pub expanded_filter: GlobFilter,

}

impl SidebarNode {
	pub fn collect(
		filter: In<GlobFilter>,
		world: &mut World,
		path_tree: Res<RoutePathTree>,
	) -> Self {
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
		Self::map_node(&filter, &info_map, path_tree)
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
