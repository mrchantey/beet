//! Charcell decorations: post-parse annotations consumed by the paint pipeline.
//!
//! These attach extra components to existing element entities rather than
//! splicing nodes into the parsed document, so the same tree drives both the
//! visitor and charcell renderers.
use crate::prelude::*;
use crate::style::BoxStyle;
use crate::style::Display;
use crate::style::LayoutStyle;
use crate::style::ListStyle;
use crate::style::common_props::ListStyleProp;
use beet_core::prelude::*;

/// An OSC-8 hyperlink target attached to an `<a>` or `<img>` element.
///
/// Populated by [`apply_hyperlinks`] in the [`PostParseTree`](crate::prelude::PostParseTree)
/// schedule and threaded through the inline flow onto each glyph's [`Cell`], so
/// the stdout buffer wraps the run in an OSC-8 sequence. The live TUI does not
/// emit it (it captures the mouse, so the terminal can't action links); there
/// clicks route through the app instead.
#[derive(Debug, Clone, Component)]
pub struct Hyperlink(pub SmolStr);

/// Generated leading content for an element, the terminal equivalent of a CSS
/// `::before` marker: list bullets/numbers, blockquote bars, the `<hr>` rule,
/// and `<img>` alt text.
///
/// Computed by [`apply_markers`] from the element's structural context without
/// mutating the parsed document. The inline flow emits it as the element's
/// first run; a block leaf like `<hr>` paints it as its sole text.
///
/// Markers are a general extension point: any system in the
/// [`DecorateSet`](crate::prelude::DecorateSet) may insert a [`Marker`] to add
/// generated content. See [`heading_hash_markers`] for an opt-in example.
#[derive(Debug, Clone, Component)]
pub struct Marker(pub SmolStr);

/// Width of the `<hr>` rule, matching the legacy visitor renderer.
const HR_RULE: &str = "────────────────────";

/// Attach a [`Hyperlink`] to every `<a>` (from `href`), `<img>` (from `src`),
/// and `<iframe>` element so the inline flow can emit OSC-8 links.
///
/// An embedded `<iframe>` can't render in the terminal, so it collapses to a
/// link: its [`Marker`] (see [`iframe_marker`]) carries the title text and this
/// hyperlink its destination, preferring the `alt-src` (a normal watch URL) over
/// the embed-only `src`.
pub fn apply_hyperlinks(mut commands: Commands, elements: ElementQuery) {
	for view in elements.iter() {
		let url = match view.tag() {
			"a" => view.attribute_string("href"),
			"img" => view.attribute_string("src"),
			"iframe" => iframe_url(&view),
			_ => continue,
		};
		if !url.is_empty() {
			commands.entity(view.entity).insert(Hyperlink(url.into()));
		}
	}
}

/// Attach a [`Marker`] to elements that carry generated content:
/// `<li>` (bullet/number), `<hr>` (rule), `<img>` (alt text), and `<select>`
/// (its selected option's label). The `<blockquote>` callout draws its own
/// thick left border via the box model, so no per-paragraph quote bar is added.
pub fn apply_markers(
	mut commands: Commands,
	ruleset: RuleSetQuery,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	tags: Query<&Element>,
	children: Query<&Children>,
	// a `<select>`'s own Value (its edited selection); element-borne, so the
	// ElementQuery's text-node value query cannot see it.
	values: Query<&Value, With<Element>>,
) {
	for view in elements.iter() {
		let marker = match view.tag() {
			"li" => list_marker(
				view.entity,
				&ruleset,
				&parents,
				&tags,
				&children,
				&elements,
			),
			"hr" => Some(HR_RULE.into()),
			"img" => Some(img_marker(&view)),
			"iframe" => Some(iframe_marker(&view)),
			"select" => Some(select_marker(&view, &elements, &values)),
			_ => None,
		};
		if let Some(marker) = marker {
			commands.entity(view.entity).insert(Marker(marker));
		}
	}
}

/// Draw internal column dividers for a `.table-vertical-borders` table on the
/// terminal. The web does this with an adjacent-sibling rule in `reset.css`,
/// which the charcell cascade can't express (no sibling combinator, only
/// child/descendant), so here every cell but the first in its row gets a left
/// border mirroring its own bottom rule — the dividers fall between columns,
/// matching the web.
pub fn apply_table_vertical_borders(
	elements: ElementQuery,
	children: Query<&Children>,
	mut box_styles: Query<&mut BoxStyle>,
) {
	for view in elements.iter() {
		if view.tag() != "table"
			|| !view.contains_class_name(&classes::TABLE_VERTICAL_BORDERS)
		{
			continue;
		}
		let mut rows = Vec::new();
		collect_cell_rows(view.entity, &elements, &children, &mut rows);
		for row in rows {
			for &cell in row.iter().skip(1) {
				if let Ok(mut box_style) = box_styles.get_mut(cell) {
					box_style.border.left = box_style.border.bottom;
					box_style.border_left = box_style.border_bottom;
				}
			}
		}
	}
}

/// Collect a table's rows as lists of cell entities (`<td>`/`<th>`), in column
/// order, descending through the `<thead>`/`<tbody>`/`<tr>` wrappers. A *row* is
/// any node whose direct children are cells, mirroring the table layout pass.
fn collect_cell_rows(
	entity: Entity,
	elements: &ElementQuery,
	children: &Query<&Children>,
	rows: &mut Vec<Vec<Entity>>,
) {
	let Ok(kids) = children.get(entity) else {
		return;
	};
	let is_cell = |entity: Entity| {
		elements
			.get(entity)
			.is_ok_and(|view| matches!(view.tag(), "td" | "th"))
	};
	let cells: Vec<Entity> = kids.iter().filter(|&kid| is_cell(kid)).collect();
	if cells.is_empty() {
		for kid in kids.iter() {
			collect_cell_rows(kid, elements, children, rows);
		}
	} else {
		rows.push(cells);
	}
}

/// Generic `<details>` disclosure on the terminal, mirroring the web's native
/// collapse. A closed details (no `open` attribute) hides everything but its
/// `<summary>` and marks the caret pointing right (`▸`); an open one keeps its
/// body and points the caret down (`▾`).
///
/// A plain `<details>` gets a left caret [`Marker`] on its summary; a sidebar
/// group (`SIDEBAR_GROUP`) instead has the markup `.sidebar-caret` glyph flipped
/// in place (the web rotates it via CSS, which the terminal can't). Either way
/// the body collapses when closed, so the disclosure is interactive on both —
/// driven by [`toggle_details_on_click`].
///
/// Built from raw queries rather than [`ElementQuery`]/`AttributeQuery` so it can
/// take `&mut Value` (to flip the sidebar caret) without a query conflict over
/// the shared `Value` access those system params hold.
pub fn apply_disclosure(
	mut commands: Commands,
	elements: Query<(Entity, &Element)>,
	class_q: Query<&Classes>,
	attributes: Query<&Attributes>,
	attr_keys: Query<&Attribute>,
	children: Query<&Children>,
	mut layouts: Query<&mut LayoutStyle>,
	mut values: Query<&mut Value>,
) {
	for (details, element) in &elements {
		if element.tag() != "details" {
			continue;
		}
		let is_sidebar = class_q.get(details).is_ok_and(|class_set| {
			class_set.contains_name(&classes::SIDEBAR_GROUP)
		});
		let open = has_open_attr(details, &attributes, &attr_keys);
		let Ok(kids) = children.get(details) else {
			continue;
		};
		for child in kids.iter() {
			let is_summary = elements
				.get(child)
				.is_ok_and(|(_, element)| element.tag() == "summary");
			if is_summary {
				if is_sidebar {
					// flip the markup caret glyph (terminal can't rotate it)
					flip_sidebar_caret(
						child,
						open,
						&class_q,
						&children,
						&mut values,
					);
				} else {
					let caret = if open { "▾ " } else { "▸ " };
					commands.entity(child).insert(Marker(caret.into()));
				}
			} else if !open {
				// collapse the body out of layout; the cascade restores its display
				// on reopen (see `toggle_details_on_click`).
				if let Ok(mut layout) = layouts.get_mut(child) {
					layout.display = Display::None;
				}
			}
		}
	}
}

/// Whether `entity` carries an `open` attribute (its presence, like HTML).
fn has_open_attr(
	entity: Entity,
	attributes: &Query<&Attributes>,
	attr_keys: &Query<&Attribute>,
) -> bool {
	attr_entity(attributes, attr_keys, entity, "open").is_some()
}

/// Point a sidebar group's `.sidebar-caret` glyph down (`▾`, open) or right
/// (`▸`, closed) by rewriting its text node, the terminal stand-in for the web's
/// CSS caret rotation.
fn flip_sidebar_caret(
	summary: Entity,
	open: bool,
	class_q: &Query<&Classes>,
	children: &Query<&Children>,
	values: &mut Query<&mut Value>,
) {
	let glyph = if open { " ▾" } else { " ▸" };
	// the caret span sits directly under the summary; find it and rewrite its
	// sole text child.
	let Ok(kids) = children.get(summary) else {
		return;
	};
	for child in kids.iter() {
		if !class_q.get(child).is_ok_and(|class_set| {
			class_set.contains_name(&classes::SIDEBAR_CARET)
		}) {
			continue;
		}
		if let Ok(text_kids) = children.get(child) {
			for text in text_kids.iter() {
				if let Ok(mut value) = values.get_mut(text) {
					value.set_if_neq(Value::str(glyph));
				}
			}
		}
	}
}

/// Observer: clicking a `<summary>` toggles its `<details>` open/closed, the
/// terminal stand-in for the web's native disclosure (which the charcell target
/// has no built-in toggle for).
///
/// A click that travelled through an `<a>` inside the summary is a link
/// activation (a sidebar branch route) and navigates instead, so only the caret
/// or a plain-summary click toggles. Toggling dirties the group's
/// [`ElementStateMap`] so the cascade re-resolves the subtree, restoring the
/// collapsed body's display when it reopens.
#[cfg(feature = "tui")]
pub fn toggle_details_on_click(
	ev: On<crate::prelude::PointerUp>,
	elements: Query<(Entity, &Element)>,
	parents: Query<&ChildOf>,
	attributes: Query<&Attributes>,
	attr_keys: Query<&Attribute>,
	mut states: Query<&mut ElementStateMap>,
	mut commands: Commands,
) {
	let summary = ev.event_target();
	if !elements
		.get(summary)
		.is_ok_and(|(_, element)| element.tag() == "summary")
	{
		return;
	}
	// a click through an inner `<a>` navigates; the caret/plain summary toggles.
	if click_through_link(
		ev.original_event_target(),
		summary,
		&elements,
		&parents,
	) {
		return;
	}
	let Some(details) = nearest_details(summary, &elements, &parents) else {
		return;
	};
	// toggle the `open` attribute (presence = open)
	match attr_entity(&attributes, &attr_keys, details, "open") {
		Some(attr) => commands.entity(attr).despawn(),
		None => {
			commands.spawn((
				AttributeOf::new(details),
				Attribute::new("open"),
				Value::Bool(true),
			));
		}
	}
	// dirty the cascade so the collapsed body re-resolves its display on reopen.
	if let Ok(mut map) = states.get_mut(details) {
		map.set_changed();
	} else {
		commands.entity(details).insert(ElementStateMap::default());
	}
}

/// Whether the path from `start` up to `summary` passes through an `<a>`, ie the
/// click landed on a link nested in the summary.
#[cfg(feature = "tui")]
fn click_through_link(
	start: Entity,
	summary: Entity,
	elements: &Query<(Entity, &Element)>,
	parents: &Query<&ChildOf>,
) -> bool {
	let mut current = start;
	loop {
		if current == summary {
			return false;
		}
		if elements.get(current).is_ok_and(|(_, el)| el.tag() == "a") {
			return true;
		}
		match parents.get(current) {
			Ok(parent) => current = parent.parent(),
			Err(_) => return false,
		}
	}
}

/// The nearest `<details>` ancestor of `summary`.
#[cfg(feature = "tui")]
fn nearest_details(
	summary: Entity,
	elements: &Query<(Entity, &Element)>,
	parents: &Query<&ChildOf>,
) -> Option<Entity> {
	let mut current = summary;
	while let Ok(parent) = parents.get(current) {
		let parent = parent.parent();
		if elements
			.get(parent)
			.is_ok_and(|(_, el)| el.tag() == "details")
		{
			return Some(parent);
		}
		current = parent;
	}
	None
}

/// The bullet (`• `) or number (`N. `) prefix for a list item, from its parent
/// list's kind and the item's position among its `<li>` siblings.
fn list_marker(
	li: Entity,
	ruleset: &RuleSetQuery,
	parents: &Query<&ChildOf>,
	tags: &Query<&Element>,
	children: &Query<&Children>,
	elements: &ElementQuery,
) -> Option<SmolStr> {
	// `list-style-type: none` (inherited, eg set on an ancestor `<nav>`) strips
	// the marker, so navigation lists read as links rather than bullets.
	if ruleset
		.resolve(li, ListStyleProp, &mut default())
		.is_ok_and(|style| style == ListStyle::None)
	{
		return None;
	}

	let parent = parents.get(li).ok()?.0;
	match tags.get(parent).ok()?.tag() {
		"ul" => Some("• ".into()),
		"ol" => {
			let start = elements
				.get_as::<OrderedListView>(parent)
				.map(|view| view.start)
				.unwrap_or(1);
			let index = children
				.get(parent)
				.ok()
				.and_then(|siblings| {
					siblings
						.iter()
						.filter(|&entity| {
							tags.get(entity)
								.map(|el| el.tag() == "li")
								.unwrap_or(false)
						})
						.position(|entity| entity == li)
				})
				.unwrap_or(0);
			Some(format!("{}. ", start + index).into())
		}
		_ => None,
	}
}

/// The closed `<select>` control's text: the selected option's label plus a
/// dropdown caret. The selected option matches the select's edited [`Value`],
/// falling back to the first option (the browser's default selection).
fn select_marker(
	view: &ElementView,
	elements: &ElementQuery,
	values: &Query<&Value, With<Element>>,
) -> SmolStr {
	let selected = values
		.get(view.entity)
		.ok()
		.and_then(|value| value.as_str().ok())
		.unwrap_or_default();
	let options = elements
		.iter_descendants_inclusive(view.entity)
		.filter(|child| child.tag() == "option")
		.collect::<Vec<_>>();
	let label = options
		.iter()
		.find(|option| !selected.is_empty() && option_value(option) == selected)
		.or_else(|| options.first())
		.map(option_label)
		.unwrap_or_default();
	format!("{label} ▾").into()
}

/// An `<option>`'s submission value: its `value` attribute, falling back to its
/// label text like a browser.
pub fn option_value(view: &ElementView) -> String {
	let value = view.attribute_string("value");
	if value.is_empty() {
		option_label(view)
	} else {
		value
	}
}

/// An `<option>`'s visible label: its inner text, falling back to its `value`
/// attribute.
pub fn option_label(view: &ElementView) -> String {
	view.inner_text
		.and_then(|(_, value)| value.as_str().ok())
		.map(|label| label.to_string())
		.unwrap_or_else(|| view.attribute_string("value"))
}

/// The `[image]:`-prefixed placeholder text for an image, using the alt text and
/// falling back to the `src`. The prefix tells the reader what the link is and is
/// part of the clickable link region.
fn img_marker(view: &ElementView) -> SmolStr {
	let alt = view.attribute_string("alt");
	if alt.is_empty() {
		format!("[image]: {}", view.attribute_string("src")).into()
	} else {
		format!("[image]: {alt}").into()
	}
}

/// The `[iframe]:`-prefixed link text for an `<iframe>` collapsed to an anchor:
/// its `title`, falling back to the destination URL when untitled. The prefix
/// tells the reader what the link is and is part of the clickable link region.
fn iframe_marker(view: &ElementView) -> SmolStr {
	let title = view.attribute_string("title");
	if title.is_empty() {
		format!("[iframe]: {}", iframe_url(view)).into()
	} else {
		format!("[iframe]: {title}").into()
	}
}

/// The destination URL for an `<iframe>` link: the `alt-src` (a normal watch
/// URL) if present, else the embed-only `src`.
fn iframe_url(view: &ElementView) -> String {
	let alt_src = view.attribute_string("alt-src");
	if alt_src.is_empty() {
		view.attribute_string("src").to_string()
	} else {
		alt_src.to_string()
	}
}

/// Prefix every heading (`<h1>`..`<h6>`) with a `#`-per-level [`Marker`],
/// echoing markdown source. Not registered by default; opt in by adding it to
/// the [`DecorateSet`](crate::prelude::DecorateSet):
///
/// ```
/// # use beet_ui::prelude::*;
/// # use beet_core::prelude::*;
/// App::new()
/// 	.add_plugins(CharcellPlugin)
/// 	.add_systems(PostParseTree, heading_hash_markers.in_set(DecorateSet));
/// ```
pub fn heading_hash_markers(
	mut commands: Commands,
	headings: Query<(Entity, &Element)>,
) {
	for (entity, element) in &headings {
		if let Some(level) = heading_level(element.tag()) {
			let marker = format!("{} ", "#".repeat(level));
			commands.entity(entity).insert(Marker(marker.into()));
		}
	}
}

/// The level of a heading tag (`h1`..`h6`), or `None` for any other tag.
fn heading_level(tag: &str) -> Option<usize> {
	match tag {
		"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => tag[1..].parse().ok(),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Render to plain text, collapsing the flex buffer to trimmed lines.
	fn render(bundle: impl Bundle) -> String {
		FlexBuffer::render_oneshot_plain(40, bundle).trim_lines()
	}

	#[beet_core::test]
	fn unordered_list_bullets() {
		render(rsx! { <ul><li>"alpha"</li><li>"beta"</li></ul> })
			.xpect_contains("• alpha")
			.xpect_contains("• beta");
	}

	#[beet_core::test]
	fn ordered_list_numbers() {
		render(rsx! { <ol><li>"first"</li><li>"second"</li></ol> })
			.xpect_contains("1. first")
			.xpect_contains("2. second");
	}

	/// The per-paragraph quote bar is gone: a quoted paragraph carries no `▌`
	/// prefix (the callout's thick left border is drawn by the box model instead).
	#[beet_core::test]
	fn blockquote_has_no_paragraph_bar() {
		render(rsx! { <blockquote><p>"quoted text"</p></blockquote> })
			.xpect_contains("quoted text")
			.xnot()
			.xpect_contains("▌");
	}

	#[beet_core::test]
	fn thematic_break_rule() {
		render(rsx! { <hr/> }).xpect_contains("────");
	}

	/// A closed `<details>` collapses its body and shows a right-pointing caret,
	/// the terminal equivalent of the web's native collapsed disclosure.
	#[beet_core::test]
	fn closed_details_collapses_with_caret() {
		render(rsx! {
			<details><summary>"Summary"</summary><p>"Body"</p></details>
		})
		.xpect_contains("▸ Summary")
		.xnot()
		.xpect_contains("Body");
	}

	/// An open `<details>` keeps its body and shows a down-pointing caret.
	#[beet_core::test]
	fn open_details_expands_with_caret() {
		render(rsx! {
			<details open><summary>"Summary"</summary><p>"Body"</p></details>
		})
		.xpect_contains("▾ Summary")
		.xpect_contains("Body");
	}

	#[beet_core::test]
	fn image_alt_text() {
		render(rsx! { <img src="image.png" alt="alt text"/> })
			.xpect_contains("[image]: alt text");
	}

	/// An `<iframe>` collapses to its `[iframe]:`-prefixed `title` as an OSC-8
	/// link targeting the `alt-src` (a normal watch URL) rather than the
	/// embed-only `src`.
	#[beet_core::test]
	fn iframe_renders_as_titled_link() {
		let out = FlexBuffer::render_oneshot(40, rsx! {
			<iframe
				src="https://www.youtube.com/embed/abc123"
				alt-src="https://youtu.be/abc123"
				title="My Talk"
			/>
		});
		out.xpect_contains("[iframe]: My Talk")
			// OSC-8 hyperlink to the watch URL, not the embed URL
			.xpect_contains("\x1b]8;;https://youtu.be/abc123\x1b\\")
			.xnot()
			.xpect_contains("embed");
	}

	/// The OSC-8 link region covers only the title glyphs, not the row-filling
	/// padding, so the terminal's hyperlink underline ends at the text rather than
	/// running to the page edge. Inspecting the link map directly catches the
	/// defect even when trailing padding would otherwise trim away on render.
	#[beet_core::test]
	fn iframe_link_ends_at_title() {
		let mut world = CharcellPlugin::world();
		let root = world
			.spawn((FlexBuffer::new(40), rsx! {
				<iframe src="https://example.com/clip" title="My Talk"/>
			}))
			.id();
		world.run_schedule(PostParseTree);
		let buffer = world.entity_mut(root).take::<FlexBuffer>().unwrap();
		// "[iframe]: My Talk" is 17 cols, left-aligned: the link covers cols 0..17.
		buffer
			.link_at(UVec2::new(16, 0))
			.xpect_eq(Some("https://example.com/clip"));
		buffer.link_at(UVec2::new(17, 0)).xpect_eq(None);
		buffer.link_at(UVec2::new(39, 0)).xpect_eq(None);
	}

	/// Without an `alt-src` the link falls back to the `src`.
	#[beet_core::test]
	fn iframe_link_falls_back_to_src() {
		FlexBuffer::render_oneshot(40, rsx! {
			<iframe src="https://example.com/clip" title="Clip"/>
		})
		.xpect_contains("\x1b]8;;https://example.com/clip\x1b\\");
	}

	#[beet_core::test]
	fn nested_list_indented() {
		// the outer item's marker sits in a left gutter, so its nested list is
		// inset one marker-width, indenting the nested bullet under the label.
		let out = FlexBuffer::render_oneshot_plain(40, rsx! {
			<ul><li>"top"<ul><li>"nested"</li></ul></li></ul>
		});
		out.lines()
			.map(|line| line.trim_end())
			.filter(|line| !line.is_empty())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["• top", "  • nested"]);
	}
}

#[cfg(all(test, feature = "tui"))]
mod disclosure_test {
	use crate::prelude::*;
	use crate::render::charcell::test_host::TestHost;
	use crate::style::material::classes;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	/// The first element with `tag` in the host tree.
	fn element_by_tag(host: &mut TestHost, tag: &str) -> Entity {
		host.app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(host.app.world())
			.find(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// Trigger a `PointerUp` on `entity`, as the hit-test would on a click.
	fn click(host: &mut TestHost, entity: Entity) {
		let pointer = host.app.world_mut().spawn_empty().id();
		host.app
			.world_mut()
			.entity_mut(entity)
			.trigger(PointerUp::new(pointer));
		host.step();
		host.step();
	}

	/// Clicking a plain `<details>` summary toggles it open and closed: the body
	/// shows/hides and the caret flips, both reversibly.
	#[beet_core::test]
	fn click_summary_toggles_details() {
		let mut host = TestHost::sized(UVec2::new(40, 12));
		host.spawn_content(rsx! {
			<details><summary>"More"</summary><p>"Body text"</p></details>
		});
		host.step();
		// closed by default: caret points right, body hidden.
		host.frame_plain().as_str().xpect_contains("▸ More");
		host.frame_plain().xnot().xpect_contains("Body text");

		// click the summary: opens — caret down, body shown.
		let summary = element_by_tag(&mut host, "summary");
		click(&mut host, summary);
		host.frame_plain().as_str().xpect_contains("▾ More");
		host.frame_plain().xpect_contains("Body text");

		// click again: collapses back (the cascade restored the body, now hidden).
		click(&mut host, summary);
		host.frame_plain().as_str().xpect_contains("▸ More");
		host.frame_plain().xnot().xpect_contains("Body text");
	}

	/// Clicking a sidebar group's caret collapses and expands it, flipping the
	/// in-place caret glyph rather than adding a left marker.
	#[beet_core::test]
	fn click_caret_toggles_sidebar_group() {
		let mut host = TestHost::sized(UVec2::new(40, 12));
		host.spawn_content(rsx! {
			<details {Classes::new([classes::SIDEBAR_GROUP])} open>
				<summary {Classes::new([classes::SIDEBAR_SUMMARY])}>
					<span {Classes::new([classes::SIDEBAR_LABEL])}>"Group"</span>
					<span {Classes::new([classes::SIDEBAR_CARET])}>" ▾"</span>
				</summary>
				<ul {Classes::new([classes::SIDEBAR_LIST])}><li>"Child link"</li></ul>
			</details>
		});
		host.step();
		// open: caret down, child visible. No left marker (sidebar flips in place).
		host.frame_plain()
			.as_str()
			.xpect_contains("▾")
			.xpect_contains("Child link");
		host.frame_plain().xnot().xpect_contains("▾ Group");

		// click the caret span: collapses — caret right, child gone.
		let caret = host
			.app
			.world_mut()
			.query::<(Entity, &Classes)>()
			.iter(host.app.world())
			.find(|(_, class_set)| {
				class_set.contains_name(&classes::SIDEBAR_CARET)
			})
			.map(|(entity, _)| entity)
			.unwrap();
		click(&mut host, caret);
		host.frame_plain().as_str().xpect_contains("▸");
		host.frame_plain().xnot().xpect_contains("Child link");
	}
}
