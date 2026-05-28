//! Sidebar widget — a nav-tree built on native `<details>`/`<summary>`, so
//! collapsible behavior works on the web with no JavaScript and in tui via the
//! style system's `<details>` rendering. The legacy `sidebar.js` is gone.
//!
//! The data type [`SidebarNode`] is a minimal tree shape with `String` paths;
//! Phase 4 of `agent/plans/beet_design.md` upgrades this to a `RoutePath`-aware
//! tree and moves it to `beet_router`.
use crate::prelude::*;
use beet_core::prelude::*;

/// Per-route metadata that flows from frontmatter into the sidebar.
///
/// Phase 4 moves this to `beet_router` as part of the markdown frontmatter
/// pipeline; for now it lives next to the widget that consumes it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SidebarInfo {
	/// Display label override (falls back to the route's prettified path).
	pub label: Option<String>,
	/// Sort order within siblings (lower comes first).
	pub order: Option<u32>,
}

/// One node in a sidebar nav tree.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub struct SidebarNode {
	/// Title-case display name.
	pub display_name: String,
	/// Full path for this node's link, or `None` if it's a group header.
	pub path: Option<String>,
	/// Child nodes.
	pub children: Vec<SidebarNode>,
	/// `true` to render expanded by default (`<details open>`).
	pub expanded: bool,
}

impl SidebarNode {
	/// All non-None paths in depth-first pre-order.
	pub fn paths(&self) -> Vec<String> {
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

/// A collapsible navigation tree, styled by `nav`/`details`/`summary` rules.
///
/// `nodes` is the tree to render. Each branch becomes a `<details>` (open when
/// its node is `expanded`); leaves become `<a>` links.
#[scene]
pub fn Sidebar(nodes: Vec<SidebarNode>) -> impl Scene {
	let items: Vec<_> = nodes
		.into_iter()
		.map(|node| sidebar_item(node, true))
		.collect();
	rsx! {
		<nav id="sidebar" {Classes::new(["sidebar", "print-hidden"])}>
			{items}
		</nav>
	}
}

/// One row in the sidebar — a link, a header, or a `<details>` group. Recursive
/// helper used by [`Sidebar`]; not its own `#[scene]` widget because the
/// recursion reads the parent's `root` context.
///
/// Returns a [`Box<dyn Scene>`] (via `.any_scene()`) because each branch of the
/// match builds a differently-shaped tree and `impl Trait` cannot unify across
/// arms.
fn sidebar_item(node: SidebarNode, root: bool) -> Box<dyn Scene> {
	let SidebarNode { display_name, path, children, expanded } = node;
	let root_class = if root { "sidebar-item-root" } else { "sidebar-item" };

	if children.is_empty() {
		match path {
			Some(path) => rsx! {
				<li {Classes::new([root_class])}>
					<a {Classes::new(["sidebar-link", "leaf"])} href=path>
						{display_name}
					</a>
				</li>
			}
			.any_scene(),
			None => rsx! {
				<li {Classes::new([root_class])}>
					<span {Classes::new(["sidebar-label"])}>{display_name}</span>
				</li>
			}
			.any_scene(),
		}
	} else {
		let child_items: Vec<_> = children
			.into_iter()
			.map(|child| sidebar_item(child, false))
			.collect();
		// build summary content separately so the `<details>` arm doesn't fork.
		let summary = match path {
			Some(path) => rsx! {
				<a {Classes::new(["sidebar-link", "branch"])} href=path>
					{display_name}
				</a>
			}
			.any_scene(),
			None => rsx! {
				<span {Classes::new(["sidebar-label", "branch"])}>{display_name}</span>
			}
			.any_scene(),
		};
		if expanded {
			rsx! {
				<li {Classes::new([root_class])}>
					<details {Classes::new(["sidebar-group"])} open>
						<summary>{summary}</summary>
						<ul>{child_items}</ul>
					</details>
				</li>
			}
			.any_scene()
		} else {
			rsx! {
				<li {Classes::new([root_class])}>
					<details {Classes::new(["sidebar-group"])}>
						<summary>{summary}</summary>
						<ul>{child_items}</ul>
					</details>
				</li>
			}
			.any_scene()
		}
	}
}
