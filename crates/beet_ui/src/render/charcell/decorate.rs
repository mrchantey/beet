//! Charcell decorations: post-parse annotations consumed by the paint pipeline.
//!
//! These attach extra components to existing element entities rather than
//! splicing nodes into the parsed document, so the same tree drives both the
//! visitor and charcell renderers.
use crate::prelude::*;
use beet_core::prelude::*;

/// An OSC-8 hyperlink target attached to an `<a>` or `<img>` element.
///
/// Populated by [`apply_hyperlinks`] in the [`PostParseTree`](crate::style::PostParseTree)
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
#[derive(Debug, Clone, Component)]
pub struct Marker(pub SmolStr);

/// Width of the `<hr>` rule, matching the legacy visitor renderer.
const HR_RULE: &str = "────────────────────";

/// Attach a [`Hyperlink`] to every `<a>` (from `href`) and `<img>` (from `src`)
/// element so the inline flow can emit OSC-8 links.
pub fn apply_hyperlinks(mut commands: Commands, elements: ElementQuery) {
	for view in elements.iter() {
		let url = match view.tag() {
			"a" => view.attribute_string("href"),
			"img" => view.attribute_string("src"),
			_ => continue,
		};
		if !url.is_empty() {
			commands.entity(view.entity).insert(Hyperlink(url.into()));
		}
	}
}

/// Attach a [`Marker`] to elements that carry generated content:
/// `<li>` (bullet/number), `<p>` inside a `<blockquote>` (quote bar),
/// `<hr>` (rule), and `<img>` (alt text).
pub fn apply_markers(
	mut commands: Commands,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	tags: Query<&Element>,
	children: Query<&Children>,
) {
	for view in elements.iter() {
		let marker = match view.tag() {
			"li" => list_marker(view.entity, &parents, &tags, &children, &elements),
			"p" => blockquote_bar(view.entity, &parents, &tags),
			"hr" => Some(HR_RULE.into()),
			"img" => Some(img_marker(&view)),
			_ => None,
		};
		if let Some(marker) = marker {
			commands.entity(view.entity).insert(Marker(marker));
		}
	}
}

/// The bullet (`• `) or number (`N. `) prefix for a list item, from its parent
/// list's kind and the item's position among its `<li>` siblings.
fn list_marker(
	li: Entity,
	parents: &Query<&ChildOf>,
	tags: &Query<&Element>,
	children: &Query<&Children>,
	elements: &ElementQuery,
) -> Option<SmolStr> {
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
							tags.get(entity).map(|el| el.tag() == "li").unwrap_or(false)
						})
						.position(|entity| entity == li)
				})
				.unwrap_or(0);
			Some(format!("{}. ", start + index).into())
		}
		_ => None,
	}
}

/// The quote-bar prefix (`▌ ` per nesting level) for a paragraph inside one or
/// more `<blockquote>`s, or `None` when not quoted.
fn blockquote_bar(
	paragraph: Entity,
	parents: &Query<&ChildOf>,
	tags: &Query<&Element>,
) -> Option<SmolStr> {
	let mut depth = 0;
	let mut current = paragraph;
	while let Ok(parent) = parents.get(current) {
		current = parent.0;
		if tags.get(current).map(|el| el.tag() == "blockquote").unwrap_or(false) {
			depth += 1;
		}
	}
	(depth > 0).then(|| "▌ ".repeat(depth).into())
}

/// The `[alt]` (or `[image: src]`) placeholder text for an image.
fn img_marker(view: &ElementView) -> SmolStr {
	let alt = view.attribute_string("alt");
	if alt.is_empty() {
		format!("[image: {}]", view.attribute_string("src")).into()
	} else {
		format!("[{alt}]").into()
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

	#[beet_core::test]
	fn blockquote_bar() {
		render(rsx! { <blockquote><p>"quoted text"</p></blockquote> })
			.xpect_contains("▌ quoted text");
	}

	#[beet_core::test]
	fn thematic_break_rule() {
		render(rsx! { <hr/> }).xpect_contains("────");
	}

	#[beet_core::test]
	fn image_alt_text() {
		render(rsx! { <img src="image.png" alt="alt text"/> })
			.xpect_contains("[alt text]");
	}
}
