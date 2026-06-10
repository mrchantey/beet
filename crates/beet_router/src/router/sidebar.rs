//! Collects a [`RouteTree`] into the target-agnostic `beet_ui` [`Sidebar`]
//! widget's render tree ([`SidebarNode`]).
//!
//! [`SidebarState`] walks the route tree, applies per-route [`SidebarInfo`]
//! overrides (label/order/expanded, sourced from markdown frontmatter via
//! [`ArticleMeta`](crate::prelude::ArticleMeta)), computes active-link +
//! auto-expansion against a `current_path`, sorts siblings in natural order, and
//! returns the `Vec<SidebarNode>` the widget renders. The widget itself emits
//! the `<nav>`/`<details>`/`<a>` DOM, so this module no longer hand-rolls bundles.
//!
//! # Example
//!
//! ```rust,no_run
//! # use beet_router::prelude::*;
//! # use beet_core::prelude::*;
//! # use beet_ui::prelude::*;
//! let state = SidebarState::new("docs/getting-started")
//!     .with_info("docs", SidebarInfo {
//!         label: Some("Documentation".into()),
//!         ..default()
//!     });
//! // let nodes = state.collect(&tree);
//! // let sidebar = world.spawn_template(rsx!{ <Sidebar nodes=nodes/> });
//! ```

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// Per-route override for a sidebar entry, sourced from markdown frontmatter
/// (the `sidebar` field of [`ArticleMeta`](crate::prelude::ArticleMeta)).
///
/// Unset fields fall back to derived defaults: the label to the route's last
/// path segment, expansion to "open when the current path is a descendant".
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SidebarInfo {
	/// Display label override. Defaults to the last path segment.
	pub label: Option<String>,
	/// Sort order within siblings. Lower values come first.
	pub order: Option<u32>,
	/// Force the branch open (`Some(true)`) or closed (`Some(false)`); `None`
	/// auto-expands when the current path is a descendant.
	pub expanded: Option<bool>,
}

/// Collector that turns a [`RouteTree`] into a [`SidebarNode`] render tree.
///
/// Holds the current page path (for active-link + auto-expansion) and per-path
/// [`SidebarInfo`] overrides.
#[derive(Debug, Clone)]
pub struct SidebarState {
	/// The current page path, used for active-link detection and auto-expansion.
	pub current_path: SmolPath,
	/// Per-path override configuration.
	pub infos: HashMap<SmolPath, SidebarInfo>,
	/// Glob filter for paths whose subtree is omitted from the nav (eg infra
	/// routes like `app-info`/`analytics`). Allows every path by default.
	pub filter: GlobFilter,
	/// Whether to prepend the synthetic "Home" entry. `true` by default; set
	/// `false` when the header already links home.
	pub show_home: bool,
}

impl SidebarState {
	/// Create a new collector targeting the given current path.
	pub fn new(current_path: impl Into<SmolPath>) -> Self {
		Self {
			current_path: current_path.into(),
			infos: HashMap::default(),
			filter: GlobFilter::default(),
			show_home: true,
		}
	}

	/// Toggle the synthetic "Home" entry. Pass `false` to omit it, eg when the
	/// page header already carries a home link.
	pub fn with_home(mut self, show_home: bool) -> Self {
		self.show_home = show_home;
		self
	}

	/// Set the override for a specific path.
	pub fn with_info(
		mut self,
		path: impl Into<SmolPath>,
		info: SidebarInfo,
	) -> Self {
		self.infos.insert(path.into(), info);
		self
	}

	/// Omit routes matching the glob `pattern` (and their subtrees) from the
	/// collected nav, eg `app-info`. Allows every path by default.
	pub fn with_exclude(mut self, pattern: &str) -> Self {
		self.filter = self.filter.with_exclude(pattern);
		self
	}

	/// Include only routes matching the glob `pattern` (and their subtrees) in the
	/// collected nav, eg `docs/**`. Allows every path by default.
	pub fn with_include(mut self, pattern: &str) -> Self {
		self.filter = self.filter.with_include(pattern);
		self
	}

	/// Collect the top-level [`SidebarNode`] list for the [`Sidebar`] widget.
	///
	/// Emits a synthetic "Home" entry followed by a recursively collected,
	/// order-sorted node for each routable child of the tree.
	pub fn collect(&self, tree: &RouteTree) -> Vec<SidebarNode> {
		let mut nodes =
			if self.show_home { vec![self.home_node()] } else { Vec::new() };
		for child in self.sort_children(tree) {
			if let Some(node) = self.collect_node(&child) {
				nodes.push(node);
			}
		}
		nodes
	}

	/// The synthetic "Home" entry linking to `/`, active when at the root path.
	fn home_node(&self) -> SidebarNode {
		SidebarNode {
			display_name: "Home".into(),
			path: Some(SmolPath::default()),
			children: Vec::new(),
			expanded: false,
			active: self.current_path.segments().is_empty(),
		}
	}

	/// Recursively collect a tree node, returning `None` when it has neither a
	/// route nor any routable children.
	fn collect_node(&self, tree: &RouteTree) -> Option<SidebarNode> {
		let path = tree.path.annotated_path();
		let info = self.infos.get(&path);
		let children: Vec<SidebarNode> = self
			.sort_children(tree)
			.iter()
			.filter_map(|child| self.collect_node(child))
			.collect();
		let has_route = tree.node().is_some();

		if children.is_empty() {
			// leaf: only render if it actually routes somewhere
			if !has_route {
				return None;
			}
			Some(SidebarNode {
				display_name: self.label(&path, info),
				path: Some(path.clone()),
				children,
				expanded: false,
				active: path == self.current_path,
			})
		} else {
			// branch: collapsible, optionally also a link if it carries a route
			let expanded = match info.and_then(|info| info.expanded) {
				Some(value) => value,
				None => self.is_ancestor_of_current(&path),
			};
			Some(SidebarNode {
				display_name: self.label(&path, info),
				path: has_route.then(|| path.clone()),
				children,
				expanded,
				active: has_route && path == self.current_path,
			})
		}
	}

	/// The display label: explicit override, else the prettified last segment.
	fn label(&self, path: &SmolPath, info: Option<&SidebarInfo>) -> String {
		info.and_then(|info| info.label.clone()).unwrap_or_else(|| {
			path.last_segment().unwrap_or("home").to_string()
		})
	}

	/// Sort children by configured order, then natural order by path,
	/// dropping any whose path is excluded.
	fn sort_children(&self, tree: &RouteTree) -> Vec<RouteTree> {
		let mut children: Vec<RouteTree> = tree
			.children
			.iter()
			.filter(|child| self.filter.passes(child.path.annotated_path()))
			.cloned()
			.collect();
		children.sort_by(|a, b| {
			let path_a = a.path.annotated_path();
			let path_b = b.path.annotated_path();
			let order = |path: &SmolPath| {
				self.infos
					.get(path)
					.and_then(|info| info.order)
					.unwrap_or(u32::MAX)
			};
			match order(&path_a).cmp(&order(&path_b)) {
				std::cmp::Ordering::Equal => {
					natural_cmp(path_a.as_ref(), path_b.as_ref())
				}
				other => other,
			}
		});
		children
	}

	/// Whether the current path is at or beneath the given path.
	fn is_ancestor_of_current(&self, path: &SmolPath) -> bool {
		let prefix = path.segments();
		prefix.is_empty() || self.current_path.segments().starts_with(&prefix)
	}
}

use std::cmp::Ordering;

/// Compare with [natural sort order](https://blog.codinghorror.com/sorting-for-humans-natural-sort-order/)
fn natural_cmp(a: &str, b: &str) -> Ordering {
	let mut a = a.chars().peekable();
	let mut b = b.chars().peekable();

	loop {
		match (a.peek(), b.peek()) {
			(None, None) => return Ordering::Equal,
			(None, _) => return Ordering::Less,
			(_, None) => return Ordering::Greater,

			(Some(c1), Some(c2))
				if c1.is_ascii_digit() && c2.is_ascii_digit() =>
			{
				// collect full number chunks
				let mut n1 = String::new();
				let mut n2 = String::new();

				while let Some(c) = a.peek() {
					if c.is_ascii_digit() {
						n1.push(*c);
						a.next();
					} else {
						break;
					}
				}

				while let Some(c) = b.peek() {
					if c.is_ascii_digit() {
						n2.push(*c);
						b.next();
					} else {
						break;
					}
				}

				// compare as numbers (fallback to length if large)
				let ord = n1.len().cmp(&n2.len()).then(n1.cmp(&n2));
				if ord != Ordering::Equal {
					return ord;
				}
			}

			_ => {
				let c1 = a.next().unwrap();
				let c2 = b.next().unwrap();
				let ord = c1.cmp(&c2);
				if ord != Ordering::Equal {
					return ord;
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Spawn the `Sidebar` widget from collected nodes and render it to HTML.
	fn render_sidebar(world: &mut World, nodes: Vec<SidebarNode>) -> String {
		let entity = world
			.spawn_template(rsx! { <Sidebar nodes=nodes/> })
			.unwrap()
			.id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(entity, world))
			.unwrap()
			.to_string()
	}

	/// Build a route tree from a spawned hierarchy.
	fn tree_of(world: &mut World, root: Entity) -> RouteTree {
		world.entity(root).get::<RouteTree>().unwrap().clone()
	}

	#[beet_core::test]
	fn natural_compare() {
		let mut v = vec!["page10", "page1", "page2"];
		v.sort_by(|a, b| natural_cmp(a, b));
		assert_eq!(v, vec!["page1", "page2", "page10"]);
	}

	#[beet_core::test]
	fn collects_home_and_leaves() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				render_action::fixed_route(
					"about",
					rsx! { <p>"about"</p> }
				),
				render_action::fixed_route(
					"docs",
					rsx! { <p>"docs"</p> }
				),
			])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("about").collect(&tree);
		// home + about + docs
		nodes.len().xpect_eq(3);
		nodes[0].display_name.as_str().xpect_eq("Home");
		nodes[1].display_name.as_str().xpect_eq("about");
		nodes[2].display_name.as_str().xpect_eq("docs");
	}

	#[beet_core::test]
	fn marks_active_leaf() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				render_action::fixed_route(
					"about",
					rsx! { <p>"about"</p> }
				),
				render_action::fixed_route(
					"docs",
					rsx! { <p>"docs"</p> }
				),
			])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("about").collect(&tree);
		// home is not active, the `about` leaf is
		nodes[0].active.xpect_false();
		nodes[1].active.xpect_true();
		nodes[2].active.xpect_false();
	}

	#[beet_core::test]
	fn marks_active_home() {
		let mut world = router_world();
		let root = world
			.spawn(children![render_action::fixed_route(
				"about",
				rsx! { <p>"about"</p> }
			)])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("").collect(&tree);
		nodes[0].active.xpect_true();
	}

	#[beet_core::test]
	fn nested_branch_auto_expands_active_path() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				render_action::fixed_route(
					"about",
					rsx! { <p>"about"</p> }
				),
				(PathPartial::new("docs"), children![
					render_action::fixed_route(
						"intro",
						rsx! { <p>"intro"</p> }
					),
					render_action::fixed_route(
						"api",
						rsx! { <p>"api"</p> }
					),
				]),
			])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("docs/intro").collect(&tree);
		// home, about, docs(branch)
		let docs = nodes.iter().find(|n| n.display_name == "docs").unwrap();
		docs.expanded.xpect_true();
		docs.path.is_none().xpect_true();
		// the active leaf is inside the branch
		docs.children
			.iter()
			.find(|n| n.display_name == "intro")
			.unwrap()
			.active
			.xpect_true();
	}

	#[beet_core::test]
	fn collapses_unrelated_branches() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				(PathPartial::new("docs"), children![
					render_action::fixed_route(
						"intro",
						rsx! { <p>"intro"</p> }
					),
				]),
				(PathPartial::new("blog"), children![
					render_action::fixed_route(
						"post1",
						rsx! { <p>"post1"</p> }
					),
				]),
			])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("docs/intro").collect(&tree);
		nodes
			.iter()
			.find(|n| n.display_name == "docs")
			.unwrap()
			.expanded
			.xpect_true();
		nodes
			.iter()
			.find(|n| n.display_name == "blog")
			.unwrap()
			.expanded
			.xpect_false();
	}

	#[beet_core::test]
	fn custom_label_override() {
		let mut world = router_world();
		let root = world
			.spawn(children![render_action::fixed_route(
				"about",
				rsx! { <p>"about"</p> }
			)])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("")
			.with_info("about", SidebarInfo {
				label: Some("About Us".into()),
				..default()
			})
			.collect(&tree);
		nodes
			.iter()
			.any(|n| n.display_name == "About Us")
			.xpect_true();
		nodes
			.iter()
			.any(|n| n.display_name == "about")
			.xpect_false();
	}

	#[beet_core::test]
	fn sort_by_order() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				render_action::fixed_route(
					"zulu",
					rsx! { <p>"zulu"</p> }
				),
				render_action::fixed_route(
					"alpha",
					rsx! { <p>"alpha"</p> }
				),
			])
			.flush();
		let tree = tree_of(&mut world, root);

		// give zulu a lower order so it sorts ahead of alpha
		let nodes = SidebarState::new("")
			.with_info("zulu", SidebarInfo {
				order: Some(0),
				..default()
			})
			.collect(&tree);
		// nodes[0] is Home, then zulu, then alpha
		nodes[1].display_name.as_str().xpect_eq("zulu");
		nodes[2].display_name.as_str().xpect_eq("alpha");
	}

	#[beet_core::test]
	fn is_ancestor_of_current() {
		let state = SidebarState::new("docs/getting-started");
		state
			.is_ancestor_of_current(&SmolPath::new("docs"))
			.xpect_true();
		state
			.is_ancestor_of_current(&SmolPath::new("docs/getting-started"))
			.xpect_true();
		state
			.is_ancestor_of_current(&SmolPath::new("blog"))
			.xpect_false();
		// root is ancestor of everything
		state
			.is_ancestor_of_current(&SmolPath::default())
			.xpect_true();
	}

	#[beet_core::test]
	fn forced_expansion() {
		let mut world = router_world();
		let root = world
			.spawn(children![(PathPartial::new("docs"), children![
				render_action::fixed_route(
					"intro",
					rsx! { <p>"intro"</p> }
				),
			])])
			.flush();
		let tree = tree_of(&mut world, root);

		// current path is NOT under docs, but force expansion
		let nodes = SidebarState::new("about")
			.with_info("docs", SidebarInfo {
				expanded: Some(true),
				..default()
			})
			.collect(&tree);
		nodes
			.iter()
			.find(|n| n.display_name == "docs")
			.unwrap()
			.expanded
			.xpect_true();
	}

	#[beet_core::test]
	fn branch_with_route_carries_path() {
		let mut world = router_world();
		let root = world
			.spawn(children![(
				render_action::fixed_route(
					"docs",
					Element::new("p").with_inner_text("docs index")
				),
				children![render_action::fixed_route(
					"intro",
					rsx! { <p>"intro"</p> }
				)],
			)])
			.flush();
		let tree = tree_of(&mut world, root);

		let nodes = SidebarState::new("docs").collect(&tree);
		let docs = nodes.iter().find(|n| n.display_name == "docs").unwrap();
		// branch carries its own route, and is active at /docs
		docs.path.is_some().xpect_true();
		docs.active.xpect_true();
	}

	/// End-to-end: render the widget from a collected tree and check the
	/// produced HTML carries `aria-current`, `<details open>`, and hrefs.
	#[beet_core::test]
	fn renders_to_html() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				render_action::fixed_route(
					"about",
					rsx! { <p>"about"</p> }
				),
				(PathPartial::new("docs"), children![
					render_action::fixed_route(
						"intro",
						rsx! { <p>"intro"</p> }
					),
				]),
			])
			.flush();
		let tree = tree_of(&mut world, root);
		let nodes = SidebarState::new("docs/intro").collect(&tree);

		let html = render_sidebar(&mut world, nodes);
		html.xpect_contains("aria-current")
			.xpect_contains("<details open")
			.xpect_contains("/about")
			.xpect_contains("/docs/intro");
	}
}
