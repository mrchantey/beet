use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

use super::query::CharcellNodeData;

/// Compute the intrinsic size of a text node.
///
/// Wraps the text to `max_width` columns and returns `(max_line_width, line_count)`.
pub fn measure_text(node: &CharcellNodeData, max_width: u32) -> UVec2 {
	let Some(value) = node.value else {
		return UVec2::ZERO;
	};
	let lines = word_wrap(&value.to_string(), max_width);
	UVec2::new(
		lines.iter().map(|l| display_width(l)).max().unwrap_or(0) as u32,
		lines.len() as u32,
	)
}

/// Paint the text content of a node into `buffer` within `content_rect`.
pub fn paint_text(
	node: &CharcellNodeData,
	content_rect: URect,
	buffer: &mut Buffer,
) -> Result {
	let Some(value) = node.value else {
		return Ok(());
	};
	let lines = word_wrap(&value.to_string(), content_rect.width());
	for (i, line) in lines.iter().enumerate() {
		let y = content_rect.min.y + i as u32;
		if y >= content_rect.max.y {
			break;
		}
		let text_align = node
			.visual
			.map(|style| style.text_align)
			.unwrap_or_default();
		let aligned = align_line(line, content_rect.width(), text_align);
		buffer.write_text(
			UVec2::new(content_rect.min.x, y),
			&aligned,
			node.visual_style().clone(),
			node.entity,
		);
	}
	Ok(())
}


// ── Word wrap ─────────────────────────────────────────────────────────────────

fn word_wrap(text: &str, max_w: u32) -> Vec<String> {
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
					lines.push(w[..max_w].to_string());
					w = &w[max_w..];
				}
				current = w.to_string();
			} else if current.len() + 1 + word.len() <= max_w {
				current.push(' ');
				current.push_str(word);
			} else {
				lines.push(std::mem::take(&mut current));
				current = word.to_string();
			}
		}
		lines.push(current);
	}
	lines
}

/// Count visible characters, skipping ANSI escape sequences.
pub fn display_width(s: &str) -> usize {
	let mut w = 0;
	let mut in_esc = false;
	for ch in s.chars() {
		match ch {
			'\x1b' => in_esc = true,
			'm' if in_esc => in_esc = false,
			_ if !in_esc => w += 1,
			_ => {}
		}
	}
	w
}

fn align_line(line: &str, width: u32, align: TextAlign) -> String {
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
		CharcellPlugin::render_oneshot_sized(UVec2::new(10, 1), bundle)
			.trim_lines()
	}
	fn render_pluses(bundle: impl Bundle) -> String {
		render(bundle).replace(" ", "+")
	}

	// ── Layout ────────────────────────────────────────────────────────────────

	#[test]
	fn text_align_left() {
		render_pluses((
			rsx! { "Hi" },
			VisualStyle::default().with_text_align(TextAlign::Left),
		))
		.xpect_snapshot();
	}

	#[test]
	fn text_align_right() {
		render_pluses((
			rsx! { "Hi" },
			VisualStyle::default().with_text_align(TextAlign::Right),
		))
		.xpect_snapshot();
	}

	#[test]
	fn text_align_center() {
		render_pluses((
			rsx! { "Hi" },
			VisualStyle::default().with_text_align(TextAlign::Center),
		))
		.xpect_snapshot();
	}

	// ── Style ─────────────────────────────────────────────────────────────────

	#[test]
	fn foreground_color() {
		let visual = VisualStyle {
			foreground: Some(Color::srgb(1., 0., 0.)),
			..VisualStyle::default()
		};
		render((rsx! { "Hi" }, visual)).xpect_snapshot();
	}

	#[test]
	fn text_underline() {
		let visual = VisualStyle {
			decoration_line: DecorationLine::underline(),
			..VisualStyle::default()
		};
		render((rsx! { "Hi" }, visual)).xpect_snapshot();
	}
}
