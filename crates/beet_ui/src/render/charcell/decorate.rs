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
/// schedule and threaded through the inline flow so the stdout [`FlexBuffer`]
/// wraps the element's run in an OSC-8 sequence. The TUI ignores it.
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
/// `<li>` (bullet/number), `<hr>` (rule), and `<img>` (alt text). The
/// `<blockquote>` callout draws its own thick left border via the box model, so
/// no per-paragraph quote bar is added.
pub fn apply_markers(
	mut commands: Commands,
	ruleset: RuleSetQuery,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	tags: Query<&Element>,
	children: Query<&Children>,
) {
	for view in elements.iter() {
		let marker = match view.tag() {
			"li" => list_marker(
				view.entity, &ruleset, &parents, &tags, &children, &elements,
			),
			"hr" => Some(HR_RULE.into()),
			"img" => Some(img_marker(&view)),
			"iframe" => Some(iframe_marker(&view)),
			_ => None,
		};
		if let Some(marker) = marker {
			commands.entity(view.entity).insert(Marker(marker));
		}
	}
}

/// Draw internal column dividers for a `.table-vertical-borders` table on the
/// terminal. The web does this with an adjacent-sibling rule in `reset.css`,
/// which the charcell cascade can't express (it has no ancestor context), so
/// here every cell but the first in its row gets a left border mirroring its own
/// bottom rule — the dividers fall between columns, matching the web.
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
	let cells: Vec<Entity> =
		kids.iter().filter(|&kid| is_cell(kid)).collect();
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
/// `<summary>` and prefixes the summary with a `▸` caret; an open one keeps its
/// body and shows a `▾` caret. The sidebar's own disclosure (its right-aligned
/// branch carets and always-expanded tree) carries `SIDEBAR_GROUP`, so it is
/// left untouched.
pub fn apply_disclosure(
	mut commands: Commands,
	elements: ElementQuery,
	children: Query<&Children>,
	mut layouts: Query<&mut LayoutStyle>,
) {
	for view in elements.iter() {
		if view.tag() != "details"
			|| view.contains_class_name(&classes::SIDEBAR_GROUP)
		{
			continue;
		}
		let open = view.attribute("open").is_some();
		let Ok(kids) = children.get(view.entity) else {
			continue;
		};
		for child in kids.iter() {
			let is_summary = elements
				.get(child)
				.is_ok_and(|child| child.tag() == "summary");
			match is_summary {
				// the caret affordance, leading the summary like a tree disclosure
				true => {
					let caret = if open { "▾ " } else { "▸ " };
					commands.entity(child).insert(Marker(caret.into()));
				}
				// a closed disclosure collapses its body out of layout
				false if !open => {
					if let Ok(mut layout) = layouts.get_mut(child) {
						layout.display = Display::None;
					}
				}
				false => {}
			}
		}
	}
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
