//! Sidebar widget — a nav-tree built on native `<details>`/`<summary>`, so
//! collapsible behavior works on the web with no JavaScript and in tui via the
//! style system's `<details>` rendering. The legacy `sidebar.js` is gone.
//!
//! [`SidebarNode`] is the render tree the widget consumes; `beet_router`'s
//! `SidebarState` collects it from a `RouteTree`, applying per-route
//! `SidebarInfo` overrides (label/order/expanded sourced from frontmatter).
use crate::prelude::*;
use beet_core::prelude::*;

/// One node in a sidebar nav tree — the render shape consumed by [`Sidebar`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub struct SidebarNode {
	/// Title-case display name.
	pub display_name: String,
	/// Route for this node's link, or `None` if it's a group header. The href
	/// is `path.with_leading_slash()`.
	pub path: Option<SmolPath>,
	/// Child nodes.
	pub children: Vec<SidebarNode>,
	/// `true` to render expanded by default (`<details open>`).
	pub expanded: bool,
	/// `true` when this node is the current page; renders `aria-current="page"`.
	pub active: bool,
}

impl SidebarNode {
	/// All non-None paths in depth-first pre-order.
	pub fn paths(&self) -> Vec<SmolPath> {
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
/// its node is `expanded`); leaves become `<a>` links, marked `aria-current`
/// when `active`.
#[scene]
pub fn Sidebar(nodes: Vec<SidebarNode>) -> impl Scene {
	let items: Vec<_> = nodes
		.into_iter()
		.map(|node| sidebar_item(node, true))
		.collect();
	rsx! {
		<nav id="sidebar" {Classes::new([classes::SIDEBAR, classes::PRINT_HIDDEN])}>
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
	let SidebarNode {
		display_name,
		path,
		children,
		expanded,
		active,
	} = node;
	let root_class = if root {
		classes::SIDEBAR_ITEM_ROOT
	} else {
		classes::SIDEBAR_ITEM
	};
	let href = path.map(|path| path.with_leading_slash());

	if children.is_empty() {
		match href {
			Some(href) => leaf_link(root_class, display_name, href, active),
			None => rsx! {
				<li {Classes::new([root_class])}>
					<span {Classes::new([classes::SIDEBAR_LABEL])}>{display_name}</span>
				</li>
			}
			.any_scene(),
		}
	} else {
		let child_items: Vec<_> = children
			.into_iter()
			.map(|child| sidebar_item(child, false))
			.collect();
		let summary = summary_content(display_name, href, active);
		// One down-caret glyph, always. On the web it's pushed to the right edge
		// (flex) and CSS rotates it to point right when the group is collapsed
		// (`details:not([open])`), so it tracks the disclosure state reactively.
		// The terminal can't rotate and always renders children, so a static
		// down-caret reads correctly there.
		let summary_row = rsx! {
			<summary {Classes::new([classes::SIDEBAR_SUMMARY])}>
				{summary}
				<span {Classes::new([classes::SIDEBAR_CARET])}>" ▾"</span>
			</summary>
		};
		// the `open` attribute can't be conditionally interpolated, so fork.
		if expanded {
			rsx! {
				<li {Classes::new([root_class])}>
					<details {Classes::new([classes::SIDEBAR_GROUP])} open>
						{summary_row}
						<ul {Classes::new([classes::SIDEBAR_LIST])}>{child_items}</ul>
					</details>
				</li>
			}
			.any_scene()
		} else {
			rsx! {
				<li {Classes::new([root_class])}>
					<details {Classes::new([classes::SIDEBAR_GROUP])}>
						{summary_row}
						<ul {Classes::new([classes::SIDEBAR_LIST])}>{child_items}</ul>
					</details>
				</li>
			}
			.any_scene()
		}
	}
}

/// A leaf `<li><a>` link, marked `aria-current="page"` when active.
fn leaf_link(
	root_class: ClassName,
	display_name: String,
	href: String,
	active: bool,
) -> Box<dyn Scene> {
	let link_classes =
		|| Classes::new([classes::SIDEBAR_LINK, ClassName::string("leaf")]);
	// `aria-current` can't be conditionally interpolated, so fork on `active`.
	if active {
		rsx! {
			<li {Classes::new([root_class])}>
				<a {link_classes()} href=href aria-current="page">{display_name}</a>
			</li>
		}
		.any_scene()
	} else {
		rsx! {
			<li {Classes::new([root_class])}>
				<a {link_classes()} href=href>{display_name}</a>
			</li>
		}
		.any_scene()
	}
}

/// The `<summary>` content for a branch: a link when the branch carries a route
/// (marked `aria-current` when active), otherwise a plain label.
fn summary_content(
	display_name: String,
	href: Option<String>,
	active: bool,
) -> Box<dyn Scene> {
	let link_classes =
		|| Classes::new([classes::SIDEBAR_LINK, classes::SIDEBAR_BRANCH]);
	match href {
		Some(href) if active => rsx! {
			<a {link_classes()} href=href aria-current="page">{display_name}</a>
		}
		.any_scene(),
		Some(href) => rsx! {
			<a {link_classes()} href=href>{display_name}</a>
		}
		.any_scene(),
		None => rsx! {
			<span {Classes::new([classes::SIDEBAR_LABEL, classes::SIDEBAR_BRANCH])}>{display_name}</span>
		}
		.any_scene(),
	}
}

#[cfg(all(test, feature = "style"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A representative nav tree: top-level branches plus mixed leaf/branch
	/// children at several depths, all expanded so every indent level is visible.
	fn nodes() -> Vec<SidebarNode> {
		fn leaf(name: &str) -> SidebarNode {
			SidebarNode {
				display_name: name.into(),
				path: Some(SmolPath::new(name)),
				..default()
			}
		}
		fn branch(name: &str, children: Vec<SidebarNode>) -> SidebarNode {
			SidebarNode {
				display_name: name.into(),
				path: None,
				children,
				expanded: true,
				..default()
			}
		}
		vec![
			leaf("home"),
			branch("docs", vec![
				leaf("intro"),
				branch("crates", vec![
					leaf("beet_core"),
					branch("nested", vec![leaf("deep")]),
				]),
				leaf("guides"),
			]),
			branch("blog", vec![leaf("post-1"), leaf("post-2")]),
		]
	}

	/// Render the sidebar to plain charcell with the Material rule set.
	fn render_charcell(nodes: Vec<SidebarNode>) -> String {
		let mut world = (
			bevy::app::TaskPoolPlugin::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::scene::ScenePlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root =
			world.spawn_scene(rsx! { <Sidebar nodes=nodes/> }).unwrap().id();
		world.entity_mut(root).insert(FlexBuffer::new(40));
		world.run_schedule(crate::parse::PostParseTree);
		world
			.entity_mut(root)
			.take::<FlexBuffer>()
			.unwrap()
			.render_plain()
	}

	/// The leading-space indent of the row whose text starts with `label`.
	fn indent_of(out: &str, label: &str) -> usize {
		let row = out
			.lines()
			.find(|line| line.trim_start().starts_with(label))
			.unwrap_or_else(|| panic!("no row for `{label}`"));
		row.len() - row.trim_start().len()
	}

	/// The terminal tree indents one fixed step per depth, applied identically to
	/// leaf and branch rows (the bug this guards against was branch rows — built
	/// from `<details>`/`<summary>` — drifting deeper than their leaf siblings).
	#[beet_core::test]
	fn charcell_indent_is_consistent() {
		let out = render_charcell(nodes());
		let indent = |label| indent_of(&out, label);
		// root rows sit flush, leaf and branch alike
		indent("home").xpect_eq(0);
		indent("docs").xpect_eq(0);
		indent("blog").xpect_eq(0);
		// a leaf and a branch at the same depth share an indent
		let step = indent("intro");
		(step > 0).xpect_true();
		indent("crates").xpect_eq(step);
		indent("guides").xpect_eq(step);
		indent("post-1").xpect_eq(step);
		// each deeper level steps in by the same unit
		indent("beet_core").xpect_eq(step * 2);
		indent("nested").xpect_eq(step * 2);
		indent("deep").xpect_eq(step * 3);
	}

	/// The branch caret is right-aligned on its summary row: the label sits at the
	/// left, the caret floated to the right edge, on one row (`docs        ▾ │`).
	/// The rail's right-border divider (and its right padding) trail every row, so
	/// the caret is the rightmost *content* rather than the last glyph.
	#[beet_core::test]
	fn charcell_caret_right_aligned() {
		let out = render_charcell(nodes());
		let row = out
			.lines()
			.find(|line| line.trim_start().starts_with("docs"))
			.unwrap();
		// strip the trailing rail divider/padding to expose the row's content
		let content = row.trim_end_matches(['│', ' ']);
		// label leads the row, caret trails it at the far right of the content
		content.trim_start().xpect_starts_with("docs");
		content.xpect_ends_with("▾");
		// the caret is pushed right, not sitting immediately beside the label
		(content.len() - "docs".len() > 4).xpect_true();
	}
}
