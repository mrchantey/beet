//! Sidebar widget — a nav-tree built on native `<details>`/`<summary>`
//! disclosure, plus a responsive rail that collapses behind the header's
//! [`MenuButton`] below [`classes::SIDEBAR_BREAKPOINT_PX`].
//!
//! The responsive behavior is authored once, as width-gated rules
//! (`sidebar_hidden`/`menu_button_visible` in `classes/sidebar.rs`) both
//! targets evaluate: the browser via the serialized `@media`, the charcell
//! cascade against its surface's [`MediaViewport`]. Only the stateful half —
//! seeding `aria-hidden` from the viewport and toggling it from the menu
//! button — needs a runtime, and it exists as twins sharing the breakpoint
//! constant: `sidebar.js` on the web, [`sync_sidebar_breakpoint`] plus the
//! generic `aria-controls` disclosure observer
//! ([`toggle_aria_controls_on_click`]) on the terminal. Keep the twins in
//! lockstep.
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
///
/// Ships its own [`SidebarScript`]: on both targets the rail collapses below
/// [`classes::SIDEBAR_BREAKPOINT_PX`] and is toggled by a [`MenuButton`] in the
/// header. Served with no `aria-hidden` attribute so the rule-only default is
/// correct before any runtime acts: the `sidebar-hidden` rule hides the rail
/// below the breakpoint unless something has set `aria-hidden="false"`, so a
/// narrow load never flashes the rail. The script then keeps the attribute in
/// sync on the web; [`sync_sidebar_breakpoint`] and the `aria-controls`
/// observer do the same on the terminal.
#[template]
pub fn Sidebar(nodes: Vec<SidebarNode>) -> impl Bundle {
	let items: Vec<_> = nodes
		.into_iter()
		.map(|node| sidebar_item(node, true))
		.collect();
	rsx! {
		<nav id="sidebar" {Classes::new([classes::SIDEBAR, classes::PRINT_HIDDEN])}>
			{items}
			<SidebarScript/>
		</nav>
	}
}

/// Emits the bundled responsive-sidebar script as an inline `<script>`, with the
/// breakpoint injected so the resize handler matches the CSS. Web-only in
/// effect (a terminal never executes scripts); its native twin is
/// [`sync_sidebar_breakpoint`] plus the `aria-controls` disclosure observer.
/// Bundled with [`Sidebar`]; standalone only so its world-free body stays out
/// of the tree match in [`sidebar_item`].
#[template]
pub fn SidebarScript() -> impl Bundle {
	let body = format!(
		"const BREAKPOINT={};\n{}",
		classes::SIDEBAR_BREAKPOINT_PX,
		include_str!("./sidebar.js"),
	);
	rsx! {
		<script>{body}</script>
	}
}

/// The sidebar toggle for the app bar — a hamburger icon button wired to
/// `#sidebar` via `aria-controls`. Hidden by default and revealed below
/// [`classes::SIDEBAR_BREAKPOINT_PX`] by the `menu-button` rules on both
/// targets; the click is bound by [`SidebarScript`] on the web and the generic
/// `aria-controls` disclosure observer on the terminal. Place it in the
/// header's leading slot, left of the title.
///
/// One hamburger glyph per target, forked by the `terminal-hidden`/
/// `terminal-only` utility classes: `☰` (U+2630) suits the web but is East
/// Asian *ambiguous-width* — a CJK-configured terminal renders it two cells
/// wide while the charcell engine paints one, smearing neighbours — and a
/// terminal cannot scale it up via `font-size`. `三` (U+4E09) is unambiguously
/// Wide: two cells on every terminal, three clean strokes, the largest
/// single-line hamburger a char grid can draw.
///
/// Classed as a text button like the app bar's other controls — plain
/// surface foreground at rest, the shared hover affordance — rather than
/// `btn-icon`, whose resting `on-surface-variant` reads permanently greyed
/// beside them.
#[template]
pub fn MenuButton() -> impl Bundle {
	rsx! {
		<button
			id="menu-button"
			aria-controls="sidebar"
			aria-label="Toggle navigation"
			{Classes::new([classes::BTN, classes::BTN_TEXT, classes::MENU_BUTTON])}>
			<span {Classes::new([classes::TERMINAL_HIDDEN])}>"☰"</span>
			<span {Classes::new([classes::TERMINAL_ONLY])}>"三"</span>
		</button>
	}
}

/// Native twin of `sidebar.js`'s init/`resize` wiring: seed the rail's
/// `aria-hidden` from the surface width, and re-seed when a resize crosses
/// [`classes::SIDEBAR_BREAKPOINT_PX`] — so the width-gated `sidebar-hidden`
/// rule and the [`MenuButton`] toggle behave exactly like the web. The click
/// itself rides the generic `aria-controls` disclosure observer
/// ([`toggle_aria_controls_on_click`]).
///
/// Mirrors the script's rules per live surface (each SSH session tracks its
/// own `wasNarrow`): only a *crossing* re-seeds, so a manual toggle survives
/// same-side resizes, and a freshly built page — its rail carrying no
/// `aria-hidden`, like a fresh DOM on page load — is seeded once; `Added`
/// elements are the cheap signal a page may have been (re)built.
// registered by `CharcellTuiPlugin` (and reads the tui-gated disclosure
// helpers), so gated like it.
#[cfg(feature = "tui")]
pub(crate) fn sync_sidebar_breakpoint(
	mut was_narrow: Local<HashMap<Entity, bool>>,
	mut commands: Commands,
	surfaces: Query<(Entity, &MediaViewport), With<DoubleBuffer>>,
	added_elements: Query<(), Added<Element>>,
	children: Query<&Children>,
	portals: Query<&Portal>,
	attributes: Query<&Attributes>,
	attr_keys: Query<&Attribute>,
	mut values: Query<&mut Value>,
	mut states: Query<&mut ElementStateMap>,
) {
	let fresh_elements = !added_elements.is_empty();
	for (surface, viewport) in surfaces.iter() {
		// at-or-below, inclusive like the `max-width` rule (and the script's
		// `narrow()`): a strict `<` would leave one width where the button
		// shows over a still-open rail.
		let narrow =
			viewport.width_px() <= classes::SIDEBAR_BREAKPOINT_PX as f32;
		let crossed = was_narrow
			.insert(surface, narrow)
			.is_none_or(|previous| previous != narrow);
		if !crossed && !fresh_elements {
			continue;
		}
		let Some(sidebar) = find_by_id(
			&children, &portals, &attributes, &attr_keys, &values, surface,
			"sidebar",
		) else {
			continue;
		};
		// a crossing re-seeds unconditionally (collapsing/restoring even a
		// manually toggled rail, like the script); a fresh page only seeds a
		// rail that carries no attribute yet.
		if !crossed
			&& attr_entity(&attributes, &attr_keys, sidebar, "aria-hidden")
				.is_some()
		{
			continue;
		}
		set_attr_str(
			&mut commands,
			&mut values,
			&mut states,
			&attributes,
			&attr_keys,
			sidebar,
			"aria-hidden",
			if narrow { "true" } else { "false" },
		);
	}
}

/// One row in the sidebar — a link, a header, or a `<details>` group. Recursive
/// helper used by [`Sidebar`]; not its own `#[template]` widget because the
/// recursion reads the parent's `root` context.
///
/// Returns a [`Snippet`] (via `.any_snippet()`) because each branch of the
/// match builds a differently-shaped tree and `impl Trait` cannot unify across
/// arms.
fn sidebar_item(node: SidebarNode, root: bool) -> Snippet {
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
			.any_snippet(),
		}
	} else {
		let child_items: Vec<_> = children
			.into_iter()
			.map(|child| sidebar_item(child, false))
			.collect();
		let summary = summary_content(display_name, href, active);
		// One down-caret glyph, pushed to the right edge (flex). The web rotates
		// it to point right when the group is collapsed (`details:not([open])`),
		// tracking the disclosure state reactively; the terminal can't rotate, so
		// `flip_sidebar_caret` rewrites the glyph with the state instead.
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
			.any_snippet()
		} else {
			rsx! {
				<li {Classes::new([root_class])}>
					<details {Classes::new([classes::SIDEBAR_GROUP])}>
						{summary_row}
						<ul {Classes::new([classes::SIDEBAR_LIST])}>{child_items}</ul>
					</details>
				</li>
			}
			.any_snippet()
		}
	}
}

/// A leaf `<li><a>` link, marked `aria-current="page"` when active.
fn leaf_link(
	root_class: ClassName,
	display_name: String,
	href: String,
	active: bool,
) -> Snippet {
	let link_classes =
		|| Classes::new([classes::SIDEBAR_LINK, ClassName::string("leaf")]);
	// `aria-current` can't be conditionally interpolated, so fork on `active`.
	if active {
		rsx! {
			<li {Classes::new([root_class])}>
				<a {link_classes()} href=href aria-current="page">{display_name}</a>
			</li>
		}
		.any_snippet()
	} else {
		rsx! {
			<li {Classes::new([root_class])}>
				<a {link_classes()} href=href>{display_name}</a>
			</li>
		}
		.any_snippet()
	}
}

/// The `<summary>` content for a branch: a link when the branch carries a route
/// (marked `aria-current` when active), otherwise a plain label.
fn summary_content(
	display_name: String,
	href: Option<String>,
	active: bool,
) -> Snippet {
	let link_classes =
		|| Classes::new([classes::SIDEBAR_LINK, classes::SIDEBAR_BRANCH]);
	match href {
		Some(href) if active => rsx! {
			<a {link_classes()} href=href aria-current="page">{display_name}</a>
		}
		.any_snippet(),
		Some(href) => rsx! {
			<a {link_classes()} href=href>{display_name}</a>
		}
		.any_snippet(),
		None => rsx! {
			<span {Classes::new([classes::SIDEBAR_LABEL, classes::SIDEBAR_BRANCH])}>{display_name}</span>
		}
		.any_snippet(),
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

	/// Render `template` to plain charcell with the Material rule set, on a
	/// surface wide enough that the rail sits above the responsive collapse
	/// breakpoint (90 columns).
	fn render_plain(
		template: impl bevy::ecs::template::Template<Output = ()>,
	) -> String {
		render_plain_sized(100, template)
	}

	/// [`render_plain`] at an explicit surface width, for tests that exercise
	/// the responsive breakpoint.
	fn render_plain_sized(
		width: u32,
		template: impl bevy::ecs::template::Template<Output = ()>,
	) -> String {
		let mut world = (
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root = world.spawn_template(template).unwrap().id();
		world.entity_mut(root).insert(FlexBuffer::new(width));
		world.run_schedule(crate::parse::PostParseTree);
		world
			.entity_mut(root)
			.take::<FlexBuffer>()
			.unwrap()
			.render_plain()
	}

	/// Render the sidebar to plain charcell with the Material rule set.
	fn render_charcell(nodes: Vec<SidebarNode>) -> String {
		render_plain(rsx! { <Sidebar nodes=nodes/> })
	}

	/// Render the sidebar into a styled [`FlexBuffer`] of fixed `width`, for cell
	/// inspection (entity ownership, background). Narrow widths sit below the
	/// responsive breakpoint, so the rail is marked `aria-hidden="false"` — the
	/// open state the seed/toggle runtimes produce live — to keep it rendered.
	fn render_sidebar_cells(width: u32, nodes: Vec<SidebarNode>) -> FlexBuffer {
		let mut world = (
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root = world
			.spawn_template(rsx! { <Sidebar nodes=nodes/> })
			.unwrap()
			.id();
		world.spawn((
			AttributeOf::new(root),
			Attribute::new("aria-hidden"),
			Value::str("false"),
		));
		world.entity_mut(root).insert(FlexBuffer::new(width));
		world.run_schedule(crate::parse::PostParseTree);
		world.entity_mut(root).take::<FlexBuffer>().unwrap()
	}

	/// The menu button is responsive on the terminal exactly like the web:
	/// hidden above the collapse breakpoint (90 columns), shown below it — and
	/// with the terminal's wide `三` glyph, never the web's ambiguous-width
	/// `☰`. Rendered beside a sibling so the wide case distinguishes "hidden"
	/// from "empty buffer".
	#[beet_core::test]
	fn menu_button_follows_breakpoint() {
		let template = || {
			rsx! {
				<div>
					<MenuButton/>
					<span>"Beet"</span>
				</div>
			}
		};
		// wide: hidden, only the sibling paints
		render_plain_sized(100, template())
			.xpect_contains("Beet")
			.xnot()
			.xpect_contains("三");
		// narrow: shown with the terminal glyph; the web `☰` span stays
		// terminal-hidden
		render_plain_sized(40, template())
			.xpect_contains("三")
			.xnot()
			.xpect_contains("☰");
	}

	/// The entity of the sole `<nav>` element.
	fn nav_entity(world: &mut World) -> Entity {
		world
			.query::<(Entity, &Element)>()
			.iter(world)
			.find(|(_, element)| element.tag() == "nav")
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// The `aria-hidden` value on the sole `<nav>`.
	fn nav_hidden(world: &mut World) -> Option<SmolStr> {
		let nav = nav_entity(world);
		world.with_state::<(
			Query<&Attributes>,
			Query<&Attribute>,
			Query<&mut Value>,
		), _>(move |(attributes, attr_keys, values)| {
			attr_string(&attributes, &attr_keys, &values, nav, "aria-hidden")
		})
	}

	/// Overwrite the seeded `aria-hidden` on the sole `<nav>`, simulating the
	/// menu-button toggle.
	fn set_nav_hidden(world: &mut World, value: &'static str) {
		let nav = nav_entity(world);
		world.with_state::<(
			Query<&Attributes>,
			Query<&Attribute>,
			Query<&mut Value>,
		), _>(move |(attributes, attr_keys, mut values)| {
			let attr = attr_entity(&attributes, &attr_keys, nav, "aria-hidden")
				.unwrap();
			*values.get_mut(attr).unwrap() = Value::str(value);
		});
	}

	/// [`sync_sidebar_breakpoint`] mirrors `sidebar.js`: seed on first sight,
	/// re-seed only when a resize crosses the breakpoint, leave a manual toggle
	/// alone on same-side resizes, and seed a freshly built rail like a page
	/// load.
	/// Spawn a `<nav id="sidebar">` under `surface` through the template
	/// substrate (which materializes its attributes).
	fn spawn_rail(world: &mut World, surface: Entity) {
		let nav = world
			.spawn_template(rsx! { <nav id="sidebar">"nav"</nav> })
			.unwrap()
			.id();
		world.entity_mut(nav).insert(ChildOf(surface));
	}

	#[beet_core::test]
	fn seeds_sidebar_across_breakpoint() {
		use bevy::math::UVec2;
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		let system = world.register_system(sync_sidebar_breakpoint);
		let surface = world
			.spawn((
				DoubleBuffer::new(UVec2::new(40, 24)),
				MediaViewport::new(640.),
			))
			.id();
		spawn_rail(&mut world, surface);
		// first sight of a narrow surface seeds hidden, like the script's init
		world.run_system(system).unwrap();
		nav_hidden(&mut world).unwrap().xpect_eq("true");
		// crossing up re-seeds shown
		world.entity_mut(surface).insert(MediaViewport::new(1600.));
		world.run_system(system).unwrap();
		nav_hidden(&mut world).unwrap().xpect_eq("false");
		// crossing down to *exactly* the breakpoint re-seeds hidden: the seed is
		// inclusive like the `max-width` rule, so no width shows the menu button
		// over a still-open rail
		world.entity_mut(surface).insert(MediaViewport::new(1024.));
		world.run_system(system).unwrap();
		nav_hidden(&mut world).unwrap().xpect_eq("true");
		// a manual toggle (the menu button opening the rail) survives a
		// same-side resize, exactly like the script's `wasNarrow` guard
		set_nav_hidden(&mut world, "false");
		world.entity_mut(surface).insert(MediaViewport::new(800.));
		world.run_system(system).unwrap();
		nav_hidden(&mut world).unwrap().xpect_eq("false");
		// a freshly built rail (a new page, no attribute yet) re-seeds
		let nav = nav_entity(&mut world);
		world.entity_mut(nav).despawn();
		spawn_rail(&mut world, surface);
		world.run_system(system).unwrap();
		nav_hidden(&mut world).unwrap().xpect_eq("true");
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

	/// A leaf link paints its full row width as one filled box (not just its
	/// label), so the whole row is the click/hover target rather than only the
	/// text. The cells past the label form one contiguous painted box carrying a
	/// background, distinct from the bare rail behind it; the label glyph sits
	/// inside that box. Guards the sidebar-anchor bug where the row beyond the text
	/// belonged to the rail, so only the text was interactive.
	#[beet_core::test]
	fn leaf_link_fills_row_width() {
		let width = 20u32;
		let buffer = render_sidebar_cells(width, nodes());
		let cells = buffer.cells();
		let row_of =
			|y: u32| &cells[(y * width) as usize..((y + 1) * width) as usize];
		// the row whose first glyph is the `home` leaf
		let row = (0u32..(cells.len() as u32 / width))
			.map(row_of)
			.find(|row| {
				row.iter()
					.find_map(|cell| cell.symbol.as_ref())
					.map(SmolStr::as_str)
					== Some("h")
			})
			.expect("a `home` row");
		// the link box fills the row past the label as one contiguous entity,
		// each cell carrying its surface background (the hover/active target).
		let link = row[10]
			.entity
			.expect("mid-row cell painted by the link box");
		row[8].entity.xpect_eq(Some(link));
		row[14].entity.xpect_eq(Some(link));
		row[10].style.background.is_some().xpect_true();
		// the label text sits inside that box (a distinct child glyph entity)
		(row[0].entity.expect("label glyph") != link).xpect_true();
		// and the box is the link, not the bare rail divider trailing the row
		(row[(width - 1) as usize].entity != Some(link)).xpect_true();
	}

	/// The resolved background of an active (`aria-current="page"`) sidebar link
	/// under `scheme`, optionally while hovered, after the style cascade settles.
	fn active_link_background(
		scheme: ClassName,
		hovered: bool,
	) -> Option<bevy::prelude::Color> {
		let mut world = (
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root = world
			.spawn_template(rsx! {
				<nav {Classes::new([scheme])}>
					<a {Classes::new([classes::SIDEBAR_LINK])} href="/x" aria-current="page">"home"</a>
				</nav>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(FlexBuffer::new(20));
		let link = world
			.query::<(Entity, &Element)>()
			.iter(&world)
			.find(|(_, element)| element.tag() == "a")
			.map(|(entity, _)| entity)
			.unwrap();
		if hovered {
			world
				.entity_mut(link)
				.insert(ElementStateMap::with(ElementState::Hovered));
		}
		world.run_schedule(crate::parse::PostParseTree);
		world
			.get::<crate::style::VisualStyle>(link)
			.unwrap()
			.background
	}

	/// The active page keeps its raised pill background while hovered, in both
	/// schemes. Regression: in the dark scheme the hover state layer redirected the
	/// background to the (dark-unset) `HoverSurface` token, which resolved to nothing
	/// and *cleared* the active row's fill — so hovering the current page in dark mode
	/// dropped its highlight ("border") instead of leaving it and only dimming the text.
	#[beet_core::test]
	fn active_link_keeps_background_on_hover() {
		// at rest both schemes paint the active pill.
		active_link_background(classes::DARK_SCHEME, false).xpect_some();
		active_link_background(classes::LIGHT_SCHEME, false).xpect_some();
		// hovered, the pill must remain (not clear to the rail) in both schemes.
		active_link_background(classes::DARK_SCHEME, true).xpect_some();
		active_link_background(classes::LIGHT_SCHEME, true).xpect_some();
	}
}
