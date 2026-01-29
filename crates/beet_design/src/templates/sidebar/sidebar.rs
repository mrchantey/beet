use crate::prelude::*;
use beet_core::prelude::*;
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
		<nav id="sidebar" class="bt-u-print-hidden" aria-hidden="false">
		{nodes.into_iter().map(|node|
			rsx!{<SidebarItem root node=node/>}).collect::<Vec<_>>()
		}
		</nav>
	  <script hoist:body src="./sidebar.js"/>
		<style>
			nav{
				--sidebar-width:15rem;
				--sidebar-indent: 0.5rem;
				background-color:var(--bt-color-surface-container-low);
				padding: 0.5.em 0.5.em 0 0;
				width: var(--sidebar-width);
				min-width: var(--sidebar-width);
				max-width: var(--sidebar-width);
				/* overflow-y:scroll; */
			}

		</style>
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Serialize, Deserialize))]
pub struct SidebarNode {
	/// A Title Case name for the group
	pub display_name: String,
	/// if this node has a route, this is its full path
	pub path: Option<RoutePath>,
	/// all paths available at this level of the tree
	pub children: Vec<SidebarNode>,
	/// expanded portions of the tree
	pub expanded: bool,
}

impl SidebarNode {
	/// Collect all paths in dfs pre-order
	pub fn paths(&self) -> Vec<RoutePath> {
		let mut paths = Vec::new();
		if let Some(path) = &self.path {
			paths.push(path.clone());
		}
		for child in &self.children {
			paths.extend(child.paths());
		}
		paths
	}
}

impl std::fmt::Display for SidebarNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let path_str = match &self.path {
			Some(p) => p.to_string(),
			None => "None".to_string(),
		};
		writeln!(
			f,
			"SidebarNode: {} ({}){}",
			self.display_name,
			path_str,
			if self.expanded { " [expanded]" } else { "" }
		)?;
		for child in &self.children {
			let child_str = format!("{}", child)
				.lines()
				.map(|line| format!("    {}", line))
				.collect::<Vec<_>>()
				.join("\n");
			writeln!(f, "{}", child_str)?;
		}
		Ok(())
	}
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
		In((this, endpoint_tree)): In<(Self, EndpointTree)>,
	) -> SidebarNode {
		this.map_node(&endpoint_tree)
	}

	pub fn map_node(&self, node: &EndpointTree) -> SidebarNode {
		let has_endpoint = node.endpoint.is_some();
		let route_path = node.pattern.annotated_route_path();
		let children = node
			.children
			.iter()
			.filter(|child| {
				self.include_filter
					.passes(&child.pattern.annotated_route_path().0)
			})
			.map(|child| self.map_node(child))
			.collect();

		// Helper to get a display name from a RoutePath
		fn pretty_route_name(route: &RoutePath) -> String {
			let str = route
				.file_name()
				.map(|name| name.to_str())
				.flatten()
				.unwrap_or("");
			if str.is_empty() {
				"Root".to_string()
			} else {
				str.to_title_case()
			}
		}

		let expanded = self.expanded_filter.passes(&route_path.0);
		let path = if has_endpoint {
			Some(route_path.clone())
		} else {
			None
		};
		let display_name = pretty_route_name(&route_path);

		SidebarNode {
			display_name,
			path,
			children,
			expanded,
		}
	}
}


impl SidebarNode {}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	#[beet_core::test]
	async fn collect_sidebar_node() {
		#[template]
		fn TestSidebar(
			entity: Entity,
			#[field(param)] route_query: RouteQuery,
		) -> Result<TextNode> {
			// Use the template entity for endpoint_tree lookup since it has
			// a proper TemplateOf/ChildOf chain to the router
			let endpoint_tree = route_query.endpoint_tree(entity)?;

			// Verify we got the endpoint tree
			endpoint_tree.to_string().xpect_eq("/docs\n");

			let sidebar_node = CollectSidebarNode {
				include_filter: GlobFilter::default(),
				expanded_filter: GlobFilter::default().with_include("/docs/"),
			}
			.map_node(&endpoint_tree);

			// Verify the sidebar node was created
			// The root of the tree has display name "Root", with /docs as a child
			sidebar_node.display_name.xpect_eq("Root");
			sidebar_node.children.len().xpect_eq(1);
			sidebar_node.children[0].display_name.xpect_eq("Docs");

			TextNode::new("Success").xok()
		}

		RouterPlugin::world()
			.spawn(router_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get()
						.with_path("docs")
						.with_action(|| (BeetRoot, rsx! {<TestSidebar/>})),
					html_bundle_to_response(),
				])
			}))
			.exchange(Request::get("/docs"))
			.await
			.unwrap_str()
			.await
			.xpect_eq("Success");
	}

	#[beet_core::test]
	async fn works() {
		#[template]
		fn TestSidebarRender() -> impl Bundle {
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
			rsx! { <Sidebar nodes=nodes /> }
		}

		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					EndpointBuilder::get().with_action(|| (
						BeetRoot,
						rsx! { <TestSidebarRender /> }
					)),
					html_bundle_to_response(),
				])
			}))
			.exchange_str(Request::get("/"))
			.await
			.xpect_contains("Partying");
	}
}
