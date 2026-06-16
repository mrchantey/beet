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

/// The document [`Head`] with a per-route `<title>`: the base [`Head`] omits its
/// own `<title>` and this widget owns the single one, bound to the route's
/// [`ArticleMeta`] title (`@entity:PageRoot::ArticleMeta.title`) so it differs
/// per route and stays live. The base head's social/PWA meta names the site from
/// [`PackageConfig`](beet_core::prelude::PackageConfig). Extra tags (stylesheet,
/// favicon, ...) flow through to the `<head>` via the default slot.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so
/// a BSX layout declares `<RouteHead>...</RouteHead>`. Builds inside a layout
/// render (it reads [`RequestContext`]).
#[template(system)]
pub fn RouteHead(
	cx: Res<RequestContext>,
	metas: Query<&ArticleMeta>,
	pkg: Res<PackageConfig>,
) -> impl Bundle {
	// the SSR seed: the resolved title rendered before any document sync, read
	// off the rendered content. The binding then keeps it live and per-route, so
	// the title is never sticky.
	let seed = metas
		.get(cx.content())
		.ok()
		.and_then(|meta| meta.title.clone())
		.unwrap_or_else(|| pkg.title.to_string());
	rsx! {
		<Head omit_title=true>
			<title>{route_title(&seed)}</title>
			<Slot/>
		</Head>
	}
}

/// The bound text child of the route `<title>`: a [`Value`] seeded with the
/// resolved title (so SSR renders it before any sync) plus, under `json`, a
/// [`ReflectFieldRef`] resolving `@entity:PageRoot::ArticleMeta.title` — the
/// nearest render-root ancestor, hopping the layout's [`LayoutContent`] into the
/// transcluded route content. Re-resolved each sync pass and each request, so the
/// title tracks the current route. Without `json` it stays the static seed.
fn route_title(seed: &str) -> impl Bundle {
	let value = Value::new(seed);
	#[cfg(feature = "json")]
	return (
		value,
		ReflectFieldRef::new("ArticleMeta", "title")
			.with_target(BindingTarget::Reserved("PageRoot".into())),
	);
	#[cfg(not(feature = "json"))]
	return value;
}

/// The route-tree navigation rail as a widget: collects the route tree this
/// page belongs to into [`Sidebar`] nodes against the current request, applying
/// each route entity's [`ArticleMeta`] (scan-time or parsed frontmatter) as its
/// [`SidebarInfo`] override.
///
/// The tree is resolved by an ancestor walk from the matched route entity
/// ([`RequestContext::route`]) rather than picking an arbitrary world
/// [`RouteTree`]: the widget is built in the detached layout subtree (the
/// content is transcluded by [`Portal`]), so its own entity has no route-tree
/// ancestor, and the rendered content may itself be a detached per-request root,
/// but the matched route entity always lives in the served tree. This scopes
/// the nav to the served site even when other [`RouteTree`]s share the world (eg
/// the `beet` CLI host's loaded command routes alongside a `beet serve` site).
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so
/// a BSX layout places it with `<RouteSidebar/>`. Builds inside a layout render
/// (it reads [`RequestContext`]); only [`PageRoute`] routes appear (so infra
/// routes like the `js/reactivity.js` asset are absent), and `exclude` adds
/// site-specific globs.
#[template(system)]
pub fn RouteSidebar(
	/// Show the synthetic `Home` entry. Disable when the header links home.
	#[prop(default = true)]
	home: bool,
	/// Glob patterns excluded from the nav, eg `["export"]`.
	#[prop]
	exclude: Vec<String>,
	cx: Res<RequestContext>,
	trees: AncestorQuery<&RouteTree>,
	metas: Query<&ArticleMeta>,
) -> impl Bundle {
	let nodes = trees
		.get(cx.route())
		.map(|tree| {
			let mut state =
				SidebarState::new(cx.current_path()).with_home(home);
			for pattern in &exclude {
				state = state.with_exclude(pattern);
			}
			// each route's metadata drives its label/order/expansion
			for node in tree.flatten_nodes() {
				if let Ok(meta) = metas.get(node.entity) {
					state = state
						.with_info(node.path.annotated_path(), meta.sidebar_info());
				}
			}
			state.collect(tree)
		})
		.unwrap_or_default();
	rsx! { <Sidebar nodes=nodes/> }
}

/// Per-route override for a sidebar entry, sourced from markdown frontmatter
/// (the `sidebar` field of [`ArticleMeta`](crate::prelude::ArticleMeta)).
///
/// Unset fields fall back to derived defaults: the label to the route's last
/// path segment, expansion to "open when the current path is a descendant".
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codegen", derive(ToTokens))]
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
		// only page routes belong in the nav; infra/data routes (eg the
		// `js/reactivity.js` asset, `app-info`) carry no [`PageRoute`] marker.
		let has_page_route =
			tree.node().is_some_and(|node| node.is_page_route);

		if children.is_empty() {
			// leaf: only render if it is a page route
			if !has_page_route {
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
			// branch: collapsible, optionally also a link if it is a page route
			let expanded = match info.and_then(|info| info.expanded) {
				Some(value) => value,
				None => self.is_ancestor_of_current(&path),
			};
			Some(SidebarNode {
				display_name: self.label(&path, info),
				path: has_page_route.then(|| path.clone()),
				children,
				expanded,
				active: has_page_route && path == self.current_path,
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

	/// A page route at `path` carrying the [`PageRoute`] marker the nav filters
	/// on. The rendered content is irrelevant to collection, which keys off the
	/// route path plus the marker, so every test route shares one body.
	fn page_route(path: &str) -> impl Bundle {
		(
			render_action::fixed_func_route(path, || rsx! { <p>"page"</p> }),
			PageRoute,
		)
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
				page_route("about"),
				page_route("docs"),
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
				page_route("about"),
				page_route("docs"),
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
			.spawn(children![page_route("about")])
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
				page_route("about"),
				(PathPartial::new("docs"), children![
					page_route("intro"),
					page_route("api"),
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
					page_route("intro"),
				]),
				(PathPartial::new("blog"), children![
					page_route("post1"),
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
			.spawn(children![page_route("about")])
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
				page_route("zulu"),
				page_route("alpha"),
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
				page_route("intro"),
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
				page_route("docs"),
				children![page_route("intro")],
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
				page_route("about"),
				(PathPartial::new("docs"), children![
					page_route("intro"),
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
