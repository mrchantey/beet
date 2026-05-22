use crate::prelude::*;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

use super::query::CharcellNodeData;

/// Compute the intrinsic size of a text node.
///
/// Wraps the text to `max_width` columns and returns `(max_line_width, line_count)`.
pub fn measure_text(node: &CharcellNodeData, max_width: u32) -> UVec2 {
	let Some(value) = node.value() else {
		return UVec2::ZERO;
	};
	let lines = word_wrap(&value.to_string(), max_width);
	UVec2::new(
		lines.iter().map(|l| display_width(l)).max().unwrap_or(0) as u32,
		lines.len() as u32,
	)
}

/// Paint text into the buffer from a [`CharcellNodeData`].
/// If the node has no [`Value`] this is a no-op.
pub(super) fn paint_text(
	node: &CharcellNodeData,
	content_rect: URect,
	buffer: &mut impl AsBuffer,
) -> Result {
	let Some(value) = node.value() else {
		return Ok(());
	};
	let text = value.to_string();
	let visual = node.visual_style();
	let entity = node.entity;
	let lines = word_wrap(&text, content_rect.width());
	for (i, line) in lines.iter().enumerate() {
		let y = content_rect.min.y + i as u32;
		if y >= content_rect.max.y {
			break;
		}
		let aligned = align_line(line, content_rect.width(), visual.text_align);
		buffer.write_text(
			UVec2::new(content_rect.min.x, y),
			&aligned,
			visual.clone(),
			entity,
		);
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
			'\x1b' => in_esc = true,
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
		Buffer::render_oneshot_sized(UVec2::new(10, 1), bundle)
			.trim_lines()
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
