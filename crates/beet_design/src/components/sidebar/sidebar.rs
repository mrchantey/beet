use crate::prelude::*;
use crate::types::ArticleMeta;
use beet_core::prelude::*;
use beet_rsx::as_beet::GlobFilter;
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


pub struct CollectSidebarNode {
	/// Which routes to include, root is always included.
	pub include_filter: GlobFilter,
	/// Set some paths to expanded by default,
	/// useful for directories without an index that dont
	/// have an `[ArticleMeta]`.
	pub expanded_filter: GlobFilter,
}

impl CollectSidebarNode {
	pub fn new(
		include_filter: GlobFilter,
		expanded_filter: GlobFilter,
	) -> Self {
		Self {
			include_filter,
			expanded_filter,
		}
	}


	pub fn collect(
		this: In<Self>,
		path_tree: Res<RoutePathTree>,
		articles: Query<&ArticleMeta>,
	) -> SidebarNode {
		this.map_node(&path_tree, &articles)
	}

	fn map_node(
		&self,
		node: &RoutePathTree,
		articles: &Query<&ArticleMeta>,
	) -> SidebarNode {
		// get the first article meta for these endpoints
		let meta = node.endpoints.iter().find_map(|e| articles.get(*e).ok());
		let contains_endpoints = node.contains_endpoints();
		let children = node
			.children
			.iter()
			.filter(|child| self.include_filter.passes(&child.route.0))
			.map(|child| self.map_node(child, articles))
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

		SidebarNode {
			display_name: meta
				.map(|m| m.sidebar.label.clone())
				.flatten()
				.unwrap_or_else(|| route_name(&node.route).to_title_case()),
			path: if contains_endpoints {
				Some(node.route.clone())
			} else {
				None
			},
			children,
			expanded: self.expanded_filter.passes(&node.route.0),
		}
	}
}


impl SidebarNode {}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[test]
	fn collect_sidebar_node() {
		let mut world = World::new();

		world.spawn((Endpoint::new(HttpMethod::Get), children![(
			RouteFilter::new("docs"),
			Endpoint::new(HttpMethod::Get),
			ArticleMeta {
				title: Some("Docs".to_string()),
				sidebar: SidebarInfo {
					label: Some("Testing".to_string()),
					order: Some(1),
				},
				..Default::default()
			},
			children![(
				RouteFilter::new("testing"),
				Endpoint::new(HttpMethod::Get),
				ArticleMeta {
					title: Some("Partying".to_string()),
					sidebar: SidebarInfo {
						label: Some("Partying".to_string()),
						order: Some(2),
					},
					..Default::default()
				},
			)]
		),]));
		world.run_system_cached(insert_route_tree).unwrap();
		world
			.run_system_cached_with(
				CollectSidebarNode::collect,
				CollectSidebarNode {
					include_filter: GlobFilter::default(), // .with_include("/docs/*")
					// .with_include("/blog/*")
					expanded_filter: GlobFilter::default()
						.with_include("/docs/"),
				},
			)
			.unwrap()
			.xpect()
			.to_be(SidebarNode {
				display_name: "Root".to_string(),
				path: Some(RoutePath::new("/")),
				children: vec![
					SidebarNode {
						display_name: "Testing".to_string(),
						path: Some(RoutePath::new("/docs")),
						children: vec![
							SidebarNode {
								display_name: "Partying".to_string(),
								path: Some(RoutePath::new("/docs/testing")),
								children: vec![],
								expanded: false,
							},
						],
						expanded: false,
					},
				],
				expanded: false,
			});
	}

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
