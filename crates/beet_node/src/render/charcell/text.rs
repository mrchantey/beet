use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

pub fn text_measure(node: &StyledNodeView, available: UVec2) -> Result<UVec2> {
	let Some(value) = node.value else {
		return Ok(UVec2::ZERO);
	};
	let lines = word_wrap(&value.to_string(), available.x);
	UVec2::new(
		lines.iter().map(|l| display_width(l)).max().unwrap_or(0) as u32,
		lines.len() as u32,
	)
	.xok()
}

pub fn text_layout(cx: &mut CharcellRenderContext) -> Result {
	let Some(value) = cx.node.value else {
		return Ok(());
	};
	let lines = word_wrap(&value.to_string(), cx.content_rect.width());
	for (i, line) in lines.iter().enumerate() {
		let y = cx.content_rect.min.y + i as u32;
		if y >= cx.content_rect.max.y {
			break;
		}
		let text_align = cx
			.node
			.layout
			.map(|style| style.text_align)
			.unwrap_or_default();
		let aligned = align_line(line, cx.content_rect.width(), text_align);
		cx.buffer.write_text(
			UVec2::new(cx.content_rect.min.x, y),
			&aligned,
			cx.node.visual_style().clone(),
			cx.node.entity,
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
				// Hard-break words longer than the column
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

/// Counts visible characters, skipping ANSI escape sequences.
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
	use bevy::math::URect;

	/// Render a bundle into a 10×5 buffer and return the plain trimmed output
	/// with spaces replaced by `+` for readable diffs.
	fn render(bundle: impl Bundle) -> String {
		// adjust if needed
		let viewport = URect::new(0, 0, 10, 1);
		RenderCharcell::new(viewport)
			.render_oneshot(bundle)
			.unwrap()
			.render()
			.trim_lines()
		// use an icon for clearer whitespace diffs
	}
	fn render_pluses(bundle: impl Bundle) -> String {
		render(bundle).replace(" ", "+")
	}

	// ── Layout ────────────────────────────────────────────────────────────────

	#[test]
	fn text_align_left() {
		render_pluses((
			rsx! { "Hi" },
			LayoutStyle::default().with_text_align(TextAlign::Left),
		))
		.xpect_snapshot();
	}

	#[test]
	fn text_align_right() {
		render_pluses((
			rsx! { "Hi" },
			LayoutStyle::default().with_text_align(TextAlign::Right),
		))
		.xpect_snapshot();
	}

	#[test]
	fn text_align_center() {
		render_pluses((
			rsx! { "Hi" },
			LayoutStyle::default().with_text_align(TextAlign::Center),
		))
		.xpect_snapshot();
	}

	// ── Style ─────────────────────────────────────────────────────────────────

	#[cfg(feature = "ansi_paint")]
	#[test]
	fn foreground_color() {
		let visual = VisualStyle {
			foreground: Some(Color::srgb(1., 0., 0.)),
			..VisualStyle::default()
		};
		render((rsx! { "Hi" }, visual)).xpect_snapshot();
	}

	#[cfg(feature = "ansi_paint")]
	#[test]
	fn text_underline() {
		let visual = VisualStyle {
			decoration_line: vec![TextDecoration::Underline],
			..VisualStyle::default()
		};
		render((rsx! { "Hi" }, visual)).xpect_snapshot();
	}
}
