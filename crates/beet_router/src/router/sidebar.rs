//! Sidebar navigation builder for [`RouteTree`].
//!
//! Builds a `<nav>` element with nested `<ul>` / `<details>` lists
//! from a route tree, suitable for site navigation. Branch nodes
//! collapse via `<details>` elements, auto-expanding when the
//! current path is a descendant.
//!
//! # Example
//!
//! ```rust,no_run
//! # use beet_router::prelude::*;
//! # use beet_core::prelude::*;
//! # use beet_node::prelude::*;
//! let state = SidebarState::new("docs/getting-started")
//!     .with_node("docs", SidebarNode {
//!         label: Some("Documentation".into()),
//!         ..default()
//!     });
//! // let sidebar = world.spawn(state.build(&tree)).flush();
//! ```

use crate::prelude::*;
use beet_core::prelude::*;
use beet_node::prelude::*;

/// Configuration for a single sidebar entry.
#[derive(Debug, Default, Clone)]
pub struct SidebarNode {
	/// Display label override. Defaults to the last path segment.
	pub label: Option<String>,
	/// Sort order within siblings. Lower values come first.
	pub order: Option<u32>,
	/// Whether children are expanded. If not set, defaults to `true`
	/// if the current path is a descendant.
	pub expanded: Option<bool>,
	/// Additional HTML attributes for this entry's anchor element.
	pub attrs: HashMap<String, Value>,
}

/// Builder for sidebar navigation from a [`RouteTree`].
///
/// Produces a `<nav>` containing nested `<ul>` lists. Leaf routes
/// render as `<a>` links; branches render as `<details>/<summary>`
/// elements that auto-expand when the current path is a descendant.
#[derive(Debug, Clone)]
pub struct SidebarState {
	/// The current page path, used for active-link detection
	/// and auto-expansion.
	pub current_path: RelPath,
	/// Per-path node configuration overrides.
	pub nodes: HashMap<RelPath, SidebarNode>,
}

impl SidebarState {
	/// Create a new sidebar state targeting the given current path.
	pub fn new(current_path: impl Into<RelPath>) -> Self {
		Self {
			current_path: current_path.into(),
			nodes: HashMap::default(),
		}
	}

	/// Set configuration for a specific path.
	pub fn with_node(
		mut self,
		path: impl Into<RelPath>,
		node: SidebarNode,
	) -> Self {
		self.nodes.insert(path.into(), node);
		self
	}

	/// Build the top-level `<nav>` element from a route tree.
	///
	/// Renders a home link followed by recursive entries for each
	/// child in the tree. The resulting bundle can be spawned directly.
	pub fn build(&self, tree: &RouteTree) -> (Element, OnSpawn) {
		let sorted = self.sort_children(tree);
		let state = self.clone();
		(
			Element::new("nav"),
			OnSpawn::new(move |entity: &mut EntityWorldMut| {
				let nav_id = entity.id();
				entity.world_scope(move |world| {
					let ul_id =
						world.spawn((Element::new("ul"), ChildOf(nav_id))).id();
					// Home link
					state.spawn_home(world, ul_id);
					// Recursive children
					for child in &sorted {
						state.spawn_child(world, child, ul_id);
					}
				});
			}),
		)
	}

	/// Spawn a "Home" link as a child of `parent_id`.
	fn spawn_home(&self, world: &mut World, parent_id: Entity) {
		let bundle = if self.current_path.segments().is_empty() {
			rsx! { <li><a href="/" aria-current="page">"Home"</a></li> }
				.any_bundle()
		} else {
			rsx! { <li><a href="/">"Home"</a></li> }.any_bundle()
		};
		world.spawn((bundle, ChildOf(parent_id)));
	}

	/// Recursively spawn a sidebar entry as a child of `parent_id`.
	fn spawn_child(
		&self,
		world: &mut World,
		tree: &RouteTree,
		parent_id: Entity,
	) {
		let path = tree.path.annotated_rel_path();
		let config = self.nodes.get(&path);
		let children = self.sort_children(tree);

		if children.is_empty() {
			// Leaf: render as link if it has a route
			if tree.node().is_some() {
				self.spawn_leaf(world, &path, config, parent_id);
			}
		} else {
			// Branch: render as collapsible details
			self.spawn_branch(world, tree, &path, config, &children, parent_id);
		}
	}

	/// Spawn a leaf link: `<li><a href="...">Label</a></li>`.
	fn spawn_leaf(
		&self,
		world: &mut World,
		path: &RelPath,
		config: Option<&SidebarNode>,
		parent_id: Entity,
	) {
		let label = config
			.and_then(|cfg| cfg.label.clone())
			.unwrap_or_else(|| Self::default_label(path));
		let href = path.with_leading_slash();
		let is_active = path == &self.current_path;

		// Wrap Value in a 1-tuple to disambiguate IntoBundle impls
		let text = (Value::Str(label.into()),);
		let bundle = if is_active {
			rsx! {
				<li><a href=href aria-current="page">{text}</a></li>
			}
			.any_bundle()
		} else {
			rsx! {
				<li><a href=href>{text}</a></li>
			}
			.any_bundle()
		};
		world.spawn((bundle, ChildOf(parent_id)));
	}

	/// Spawn a branch:
	/// `<li><details open?><summary>...</summary><ul>...</ul></details></li>`.
	fn spawn_branch(
		&self,
		world: &mut World,
		tree: &RouteTree,
		path: &RelPath,
		config: Option<&SidebarNode>,
		children: &[RouteTree],
		parent_id: Entity,
	) {
		let label = config
			.and_then(|cfg| cfg.label.clone())
			.unwrap_or_else(|| Self::default_label(path));

		// Determine expansion state
		let is_expanded = match config.and_then(|cfg| cfg.expanded) {
			Some(value) => value,
			None => self.is_ancestor_of_current(path),
		};

		// Outer structure stays imperative since children are dynamic
		let li_id = world.spawn((Element::new("li"), ChildOf(parent_id))).id();
		let details_id =
			world.spawn((Element::new("details"), ChildOf(li_id))).id();
		if is_expanded {
			world.spawn((
				Attribute::new("open"),
				Value::Bool(true),
				AttributeOf::new(details_id),
			));
		}

		// Use rsx! for the summary content.
		// Wrap Value in a 1-tuple to disambiguate IntoBundle impls.
		let text = (Value::Str(label.into()),);
		let summary_bundle = if tree.node().is_some() {
			let href = path.with_leading_slash();
			if path == &self.current_path {
				rsx! {
					<summary><a href=href aria-current="page">{text}</a></summary>
				}
				.any_bundle()
			} else {
				rsx! {
					<summary><a href=href>{text}</a></summary>
				}
				.any_bundle()
			}
		} else {
			rsx! {
				<summary>{text}</summary>
			}
			.any_bundle()
		};
		world.spawn((summary_bundle, ChildOf(details_id)));

		// Custom attributes on the details element
		if let Some(config) = config {
			Self::spawn_custom_attrs(world, details_id, config);
		}

		// Nested list for children
		let ul_id = world.spawn((Element::new("ul"), ChildOf(details_id))).id();
		for child in children {
			self.spawn_child(world, child, ul_id);
		}
	}

	/// Spawn custom attributes from config onto an element.
	fn spawn_custom_attrs(
		world: &mut World,
		element_id: Entity,
		config: &SidebarNode,
	) {
		for (key, value) in &config.attrs {
			world.spawn((
				Attribute::new(key.clone()),
				value.clone(),
				AttributeOf::new(element_id),
			));
		}
	}

	/// Sort children by configured order, then alphabetically by path.
	fn sort_children(&self, tree: &RouteTree) -> Vec<RouteTree> {
		let mut children = tree.children.clone();
		children.sort_by(|a, b| {
			let path_a = a.path.annotated_rel_path();
			let path_b = b.path.annotated_rel_path();
			let order_a = self
				.nodes
				.get(&path_a)
				.and_then(|node| node.order)
				.unwrap_or(u32::MAX);
			let order_b = self
				.nodes
				.get(&path_b)
				.and_then(|node| node.order)
				.unwrap_or(u32::MAX);
			match order_a.cmp(&order_b) {
				std::cmp::Ordering::Equal => {
					natural_cmp(path_a.as_ref(), path_b.as_ref())
				}
				other => other,
			}
		});
		children
	}

	/// Extract a display label from the last path segment.
	fn default_label(path: &RelPath) -> String {
		path.last_segment().unwrap_or("home").to_string()
	}

	/// Check if the current path is at or beneath the given path.
	fn is_ancestor_of_current(&self, path: &RelPath) -> bool {
		let current = self.current_path.segments();
		let prefix = path.segments();
		if prefix.is_empty() {
			return true;
		}
		current.starts_with(&prefix)
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

	/// Render an entity tree to an HTML string.
	fn render_html(world: &mut World, entity: Entity) -> String {
		HtmlRenderer::new()
			.render(&mut RenderContext::new(entity, world))
			.unwrap()
			.to_string()
	}
	#[test]
	fn natural_compare() {
		let mut v = vec!["page10", "page1", "page2"];
		v.sort_by(|a, b| natural_cmp(a, b));
		assert_eq!(v, vec!["page1", "page2", "page10"]);
	}
	#[test]
	fn builds_basic_sidebar() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				fixed_scene("about", rsx! { <p>"about"</p> }),
				fixed_scene("docs", rsx! { <p>"docs"</p> }),
			])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let state = SidebarState::new("about");
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		html.xpect_contains("Home")
			.xpect_contains("/about")
			.xpect_contains("/docs")
			.xpect_contains("<nav>")
			.xpect_contains("<ul>");
	}

	#[test]
	fn marks_active_leaf() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				fixed_scene("about", rsx! { <p>"about"</p> }),
				fixed_scene("docs", rsx! { <p>"docs"</p> }),
			])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let state = SidebarState::new("about");
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		html.xpect_contains("aria-current");
	}

	#[test]
	fn marks_active_home() {
		let mut world = router_world();
		let root = world
			.spawn(children![fixed_scene("about", rsx! { <p>"about"</p> })])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// Current path is root
		let state = SidebarState::new("");
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		// Home link should have aria-current
		html.xpect_contains("aria-current");
	}

	#[test]
	fn builds_nested_branches() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				fixed_scene("about", rsx! { <p>"about"</p> }),
				(PathPartial::new("docs"), children![
					fixed_scene("intro", rsx! { <p>"intro"</p> }),
					fixed_scene("api", rsx! { <p>"api"</p> }),
				]),
			])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// Current path is inside docs, so docs branch should expand
		let state = SidebarState::new("docs/intro");
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		html.xpect_contains("<details")
			.xpect_contains("<summary>")
			.xpect_contains("open")
			.xpect_contains("/docs/intro")
			.xpect_contains("/docs/api");
	}

	#[test]
	fn collapses_unrelated_branches() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				(PathPartial::new("docs"), children![fixed_scene(
					"intro",
					rsx! { <p>"intro"</p> }
				),]),
				(PathPartial::new("blog"), children![fixed_scene(
					"post1",
					rsx! { <p>"post1"</p> }
				),]),
			])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// At docs/intro, blog should be collapsed
		let state = SidebarState::new("docs/intro");
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		// docs branch should be open, blog should not
		// Both branches have <details>, but only docs has open
		html.xpect_contains("<details open");
	}

	#[test]
	fn custom_label_override() {
		let mut world = router_world();
		let root = world
			.spawn(children![fixed_scene("about", rsx! { <p>"about"</p> })])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let state = SidebarState::new("").with_node("about", SidebarNode {
			label: Some("About Us".into()),
			..default()
		});
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		html.xpect_contains("About Us")
			.xnot()
			.xpect_contains(">about<");
	}

	#[test]
	fn sort_by_order() {
		let mut world = router_world();
		let root = world
			.spawn(children![
				fixed_scene("zulu", rsx! { <p>"zulu"</p> }),
				fixed_scene("alpha", rsx! { <p>"alpha"</p> }),
			])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// Give zulu a lower order so it appears first despite alphabetical
		let state = SidebarState::new("").with_node("zulu", SidebarNode {
			order: Some(0),
			..default()
		});
		let sorted = state.sort_children(&tree);
		sorted[0]
			.path
			.annotated_rel_path()
			.last_segment()
			.unwrap()
			.xpect_eq("zulu");
		sorted[1]
			.path
			.annotated_rel_path()
			.last_segment()
			.unwrap()
			.xpect_eq("alpha");
	}

	#[test]
	fn is_ancestor_of_current() {
		let state = SidebarState::new("docs/getting-started");
		state
			.is_ancestor_of_current(&RelPath::new("docs"))
			.xpect_true();
		state
			.is_ancestor_of_current(&RelPath::new("docs/getting-started"))
			.xpect_true();
		state
			.is_ancestor_of_current(&RelPath::new("blog"))
			.xpect_false();
		// Root is ancestor of everything
		state
			.is_ancestor_of_current(&RelPath::default())
			.xpect_true();
	}

	#[test]
	fn forced_expansion() {
		let mut world = router_world();
		let root = world
			.spawn(children![(PathPartial::new("docs"), children![
				fixed_scene("intro", rsx! { <p>"intro"</p> }),
			])])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// Current path is NOT under docs, but force expansion
		let state = SidebarState::new("about").with_node("docs", SidebarNode {
			expanded: Some(true),
			..default()
		});
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		html.xpect_contains("<details open");
	}

	#[test]
	fn branch_with_route_renders_link_in_summary() {
		let mut world = router_world();
		let root = world
			.spawn(children![(
				fixed_scene(
					"docs",
					// Use with_inner_text here because rsx! produces
					// a children![] that conflicts with the outer children![]
					Element::new("p").with_inner_text("docs index")
				),
				children![fixed_scene("intro", rsx! { <p>"intro"</p> })],
			)])
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let state = SidebarState::new("docs/intro");
		let sidebar_id = world.spawn(state.build(&tree)).flush();

		let html = render_html(&mut world, sidebar_id);
		// Summary should contain an anchor link
		html.xpect_contains("<summary>").xpect_contains("/docs");
	}
}
