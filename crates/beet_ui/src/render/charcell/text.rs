use crate::prelude::*;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::IVec2;
use bevy::math::UVec2;

use super::align_offset;
use super::query::CharcellNodeData;
use super::truncate_to_width;

/// Compute the intrinsic size of a text node.
///
/// Wraps the text to `max_width` columns and returns `(max_line_width, line_count)`.
pub fn measure_text(node: &CharcellNodeData, max_width: u32) -> UVec2 {
	let Some(value) = node.value() else {
		return UVec2::ZERO;
	};
	measure_str(&value.to_string(), max_width)
}

/// Wrap `text` to `max_width` columns and return `(max_line_width, line_count)`.
pub(super) fn measure_str(text: &str, max_width: u32) -> UVec2 {
	let lines = word_wrap(text, max_width);
	UVec2::new(
		lines.iter().map(|l| display_width(l)).max().unwrap_or(0) as u32,
		lines.len() as u32,
	)
}

/// Paint text into the buffer from a [`CharcellNodeData`].
///
/// Uses the node's generated [`Marker`] (eg the `<hr>` rule or a `<select>`'s
/// label — generated content replaces a raw [`Value`], which for a form
/// control is submission state, not display text), else its [`Value`]; a
/// no-op when it has neither.
pub(super) fn paint_text(
	node: &CharcellNodeData,
	content_rect: IRect,
	buffer: &mut impl AsBuffer,
	clip: Clip,
) -> Result {
	let text = match (node.marker(), node.value()) {
		(Some(marker), _) => marker.to_string(),
		(None, Some(value)) => value.to_string(),
		(None, None) => return Ok(()),
	};
	let visual = node.visual_style();
	let entity = node.entity;
	// a marker-only leaf can still carry a link (eg an `<iframe>`/`<img>` collapsed
	// to its title/alt text); wrap its painted cells in the OSC-8 link, matching
	// the inline flow.
	let link = node.hyperlink();
	// a text decoration (eg the iframe link's underline) must underline only the
	// glyphs, not the row-filling padding, so the line is painted in two passes:
	// the full aligned line with the decoration stripped, then the glyphs alone
	// with the decoration. Skipped (single pass) when there's nothing to decorate.
	let decorated = visual.decoration_line != DecorationLine::DEFAULT;
	let undecorated =
		visual.clone().with_decoration_line(DecorationLine::DEFAULT);
	let width = content_rect.width().max(0) as u32;
	let lines = word_wrap(&text, width);
	for (i, line) in lines.iter().enumerate() {
		let y = content_rect.min.y + i as i32;
		if y >= content_rect.max.y {
			break;
		}
		let aligned = align_line(line, width, visual.text_align);
		let origin = IVec2::new(content_rect.min.x, y);
		// the glyph columns this row actually paints, used by both the decorated
		// overlay and the OSC-8 link so neither bleeds into the padding.
		let glyphs = truncate_to_width(line, width as usize);
		let glyph_width = display_width(glyphs) as u32;
		let offset = align_offset(glyph_width, width, visual.text_align);
		if decorated {
			buffer.write_text(origin, &aligned, undecorated.clone(), entity, clip);
			buffer.write_text(
				IVec2::new(content_rect.min.x + offset as i32, y),
				glyphs,
				visual.clone(),
				entity,
				clip,
			);
		} else {
			buffer.write_text(origin, &aligned, visual.clone(), entity, clip);
		}
		// the link covers only the painted glyph columns, not the row-filling
		// padding, so the terminal's hyperlink underline ends at the text.
		if let Some(link) = link {
			let start = (content_rect.min.x + offset as i32).max(0) as u32;
			for col in start..start + glyph_width {
				buffer.set_link(UVec2::new(col, y.max(0) as u32), link);
			}
		}
	}
	Ok(())
}


// ── Word wrap ─────────────────────────────────────────────────────────────────

/// Split `text` at the first column boundary that reaches `max_cols`.
fn split_at_display_width(text: &str, max_cols: usize) -> (&str, &str) {
	let mut width = 0;
	let mut byte_idx = text.len();
	for (i, ch) in text.char_indices() {
		let w = unicode_width(ch) as usize;
		if width + w > max_cols {
			byte_idx = i;
			break;
		}
		width += w;
	}
	(&text[..byte_idx], &text[byte_idx..])
}

pub(super) fn word_wrap(text: &str, max_w: u32) -> Vec<String> {
	let max_w = max_w as usize;
	if max_w == 0 {
		return vec![];
	}
	let mut lines = Vec::new();

	for para in text.split('\n') {
		let mut current = String::new();
		for word in para.split_whitespace() {
			if current.is_empty() {
				// hard-break words longer than the column
				let mut w = word;
				while display_width(w) > max_w {
					let (head, tail) = split_at_display_width(w, max_w);
					lines.push(head.to_string());
					w = tail;
				}
				current = w.to_string();
			} else if display_width(&current) + 1 + display_width(word) <= max_w
			{
				current.push(' ');
				current.push_str(word);
			} else {
				lines.push(std::mem::take(&mut current));
				current = word.to_string();
			}
		}
		// Preserve trailing whitespace from original paragraph
		if para.ends_with(|c: char| c.is_whitespace()) && !current.is_empty() {
			current.push(' ');
		}
		lines.push(current);
	}
	lines
}

/// Count visible columns, skipping ANSI escape sequences.
///
/// Wide (CJK/fullwidth) characters count as 2 columns.
pub fn display_width(s: &str) -> usize {
	let mut w = 0;
	let mut in_esc = false;
	for ch in s.chars() {
		match ch {
			escape::ESC => in_esc = true,
			'm' if in_esc => in_esc = false,
			_ if in_esc => {}
			_ => w += unicode_width(ch) as usize,
		}
	}
	w
}

pub(super) fn align_line(line: &str, width: u32, align: TextAlign) -> String {
	let w = width as usize;
	let len = display_width(line);
	if len >= w {
		return line.chars().take(w).collect();
	}
	let pad = w - len;
	match align {
		TextAlign::Left => format!("{line:<w$}"),
		TextAlign::Right => format!("{line:>w$}"),
		TextAlign::Center => {
			let l = pad / 2;
			format!("{}{line}{}", " ".repeat(l), " ".repeat(pad - l))
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	/// Render a bundle into a 10×1 buffer and return the ANSI output.
	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_sized(UVec2::new(10, 1), bundle).trim_lines()
	}
	fn render_pluses(bundle: impl Bundle) -> String {
		render(bundle).replace(" ", "+")
	}

	// ── Layout ────────────────────────────────────────────────────────────────

	#[beet_core::test]
	fn text_align_left() {
		render_pluses((
			rsx! { "Hi" },
			VisualStyle::default().with_text_align(TextAlign::Left),
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn text_align_right() {
		render_pluses((
			rsx! { "Hi" },
			VisualStyle::default().with_text_align(TextAlign::Right),
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn text_align_center() {
		render_pluses((
			rsx! { "Hi" },
			VisualStyle::default().with_text_align(TextAlign::Center),
		))
		.xpect_snapshot();
	}

	// ── Style ─────────────────────────────────────────────────────────────────

	#[beet_core::test]
	fn foreground_color() {
		let visual = VisualStyle {
			foreground: Some(Color::srgb(1., 0., 0.)),
			..VisualStyle::default()
		};
		render((rsx! { "Hi" }, visual)).xpect_snapshot();
	}

	#[beet_core::test]
	fn text_underline() {
		let visual = VisualStyle {
			decoration_line: DecorationLine::underline(),
			..VisualStyle::default()
		};
		render((rsx! { "Hi" }, visual)).xpect_snapshot();
	}

	// ── Wide character support ────────────────────────────────────────────────

	#[beet_core::test]
	fn wide_char_display_width() {
		// Each CJK character = 2 columns
		display_width("中文").xpect_eq(4);
		display_width("日本語").xpect_eq(6);
		display_width("ＡＢＣ").xpect_eq(6);
		// ASCII is 1 column each
		display_width("abc").xpect_eq(3);
	}
}
