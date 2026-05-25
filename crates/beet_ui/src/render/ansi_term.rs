use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Renders an entity tree to styled ANSI terminal output via the
/// [`charcell`](crate::render::charcell) layout engine.
///
/// The tree is painted into an auto-growing [`FlexBuffer`] — block elements
/// stack with a blank-row gap, inline elements flow and wrap at the terminal
/// width, list bullets / quote bars / rules / image alt text arrive as
/// [`Marker`] generated content, and `<a>`/`<img>` links emit
/// [OSC-8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda).
///
/// The world must have a [`CharcellPlugin`] (which pulls in [`StylePlugin`]):
/// rendering re-runs the [`PostParseTree`] schedule to decorate and paint the
/// flex buffer, so without those systems registered the buffer stays blank.
/// All styling comes from the resolved [`VisualStyle`] components: prose
/// defaults (`em` → italic, `h1` → bold colour) come from
/// [`default_element_rules`](crate::style::default_element_rules) and
/// tree-sitter syntax highlighting from `apply_syntax_highlighting`'s
/// `class="hl-<capture>"` spans. The renderer holds no style table of its own —
/// it simply paints the style resolved for each entity.
#[derive(Debug, Clone, PartialEq)]
pub struct AnsiTermRenderer {
	/// Whether to erase the screen before rendering, defaults to `false`.
	///
	/// Off by default so one-shot output (`beet --help`) prints inline and
	/// stays in the scrollback. An interactive loop that repaints in place
	/// opts in via [`Self::with_clear_on_render`].
	clear_on_render: bool,
	/// A string prepended to the output, defaults to `\n`.
	prefix: Cow<'static, str>,
}

impl Default for AnsiTermRenderer {
	fn default() -> Self { Self::new() }
}

impl AnsiTermRenderer {
	pub fn new() -> Self {
		Self {
			clear_on_render: false,
			prefix: "\n".into(),
		}
	}

	/// Override whether to erase the screen before rendering.
	pub fn with_clear_on_render(mut self, clear: bool) -> Self {
		self.clear_on_render = clear;
		self
	}

	/// Paint the tree rooted at `entity` into a [`FlexBuffer`] and return the
	/// assembled ANSI string (clear + prefix + body).
	fn render_to_string(&self, entity: Entity, world: &mut World) -> Result<String> {
		let width = terminal_ext::size().x.max(1);
		world.entity_mut(entity).insert(FlexBuffer::new(width));

		// styles were resolved when the tree was parsed; re-run the post-parse
		// pipeline to decorate and paint the freshly-inserted flex buffer. A
		// no-op when the world has no `CharcellPlugin` (structure stays unstyled).
		let _ = world.try_run_schedule(PostParseTree);

		let body = world
			.entity_mut(entity)
			.take::<FlexBuffer>()
			.unwrap()
			.render();

		let mut out = String::new();
		if self.clear_on_render {
			out.push_str(escape::CLEAR_ALL);
		}
		out.push_str(&self.prefix);
		out.push_str(&body);
		out.xok()
	}
}

impl NodeRenderer for AnsiTermRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError> {
		cx.check_accepts(&[MediaType::AnsiTerm, MediaType::Text])?;
		let output = self.render_to_string(cx.entity, cx.world)?;
		RenderOutput::media_string(MediaType::AnsiTerm, output).xok()
	}
}


#[cfg(test)]
#[cfg(feature = "markdown_parser")]
mod test {
	use super::*;

	/// Parse markdown, resolve styles, then render via [`AnsiTermRenderer`].
	fn render(md: &str) -> String {
		let mut app = App::new();
		app.add_plugins(CharcellPlugin);
		let entity = app.world_mut().spawn_empty().id();
		let bytes = MediaBytes::new_markdown(md);
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(entity),
				&bytes,
			))
			.unwrap();
		AnsiTermRenderer::new()
			.with_clear_on_render(false)
			.render(&mut RenderContext::new(entity, app.world_mut()))
			.unwrap()
			.to_string()
	}

	/// Strip ANSI escape sequences (CSI and OSC-8), leaving visible text.
	fn strip_ansi(input: String) -> String {
		let mut result = String::new();
		let mut chars = input.chars().peekable();
		while let Some(ch) = chars.next() {
			if ch == '\x1b' {
				match chars.peek() {
					// OSC sequence: ESC ] ... ST (ST = ESC \)
					Some(']') => {
						chars.next();
						loop {
							match chars.next() {
								Some('\x1b') => {
									chars.next(); // consume the \ of ST
									break;
								}
								Some('\x07') | None => break,
								_ => {}
							}
						}
					}
					// CSI sequence: ESC [ ... final byte
					Some('[') => {
						chars.next();
						loop {
							match chars.next() {
								Some(ch) if ch.is_ascii_alphabetic() => break,
								None => break,
								_ => {}
							}
						}
					}
					_ => {
						chars.next();
					}
				}
			} else {
				result.push(ch);
			}
		}
		result
	}

	#[beet_core::test]
	fn render_paragraph() {
		render("Hello world")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("Hello world");
	}

	#[beet_core::test]
	fn render_heading_text() {
		// headings carry no `#` markers by default; only the text is emitted
		render("# Title").xmap(strip_ansi).trim().xpect_eq("Title");
	}

	#[beet_core::test]
	fn render_heading_styled() {
		render("# Title")
			.xpect_contains("\x1b[")
			.xpect_contains("Title");
	}

	#[beet_core::test]
	fn render_link_has_osc8() {
		render("[click](https://example.com)")
			.xpect_contains("\x1b]8;;https://example.com\x1b\\")
			.xpect_contains("click")
			.xpect_contains("\x1b]8;;\x1b\\");
	}

	#[beet_core::test]
	fn render_link_text_stripped() {
		render("[click](https://example.com)")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("click");
	}

	#[beet_core::test]
	fn render_code_block() {
		render("```rust\nfn main() {}\n```")
			.xmap(strip_ansi)
			.xpect_contains("fn main() {}");
	}

	#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn render_code_block_syntax_styled() {
		// the `fn` keyword resolves to a syntax-highlight foreground colour
		// through the default theme, with no per-renderer configuration
		render("```rust\nfn main() {}\n```").xpect_contains("\x1b[");
	}

	#[beet_core::test]
	fn render_unordered_list() {
		render("- alpha\n- beta")
			.xmap(strip_ansi)
			.trim()
			.xpect_contains("• alpha")
			.xpect_contains("• beta");
	}

	#[beet_core::test]
	fn render_image() {
		render("![alt text](image.png)")
			.xmap(strip_ansi)
			.xpect_contains("[alt text]");
	}

	#[beet_core::test]
	fn render_image_has_osc8() {
		render("![alt](image.png)").xpect_contains("\x1b]8;;image.png\x1b\\");
	}

	#[beet_core::test]
	fn render_blockquote() {
		render("> quoted text")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ quoted text");
	}

	#[beet_core::test]
	fn render_blockquote_with_emphasis() {
		// inline elements inside a blockquote must appear after the prefix
		render("> *notable remark*")
			.xmap(strip_ansi)
			.trim()
			.xpect_eq("▌ notable remark");
	}

	#[beet_core::test]
	fn render_thematic_break() {
		render("---").xmap(strip_ansi).xpect_contains("────");
	}

	#[beet_core::test]
	fn render_multiple_blocks_separated() {
		render("# Title\n\nParagraph")
			.xmap(strip_ansi)
			.xpect_contains("Title")
			.xpect_contains("Paragraph")
			.xpect_contains("\n\n");
	}

	#[cfg(feature = "html_parser")]
	#[beet_core::test]
	fn unescape_html_entities() {
		let mut app = App::new();
		app.add_plugins(CharcellPlugin);
		let entity = app.world_mut().spawn_empty().id();
		let bytes = MediaBytes::new_html("<p>a &amp; b</p>");
		HtmlParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(entity),
				&bytes,
			))
			.unwrap();
		AnsiTermRenderer::new()
			.with_clear_on_render(false)
			.render(&mut RenderContext::new(entity, app.world_mut()))
			.unwrap()
			.to_string()
			.xmap(strip_ansi)
			.xpect_contains("a & b");
	}
}
