use crate::prelude::*;
use beet_core::prelude::*;

/// Renders an entity tree as plain text, stripping all markup: the prose, each
/// block element (and each table row, disclosure summary and `<br>`) on lines
/// of its own, inline elements flowing into the line around them, and two
/// elements with nothing between them (nav links, table cells) a space apart.
///
/// When `plaintext_only` is `true`, only [`MediaType::Text`] is accepted
/// in the `accepts` list. When `false` (the default), any text-based media
/// type is accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainTextRenderer {
	did_newline: bool,
	/// Whether an inline element just ended with nothing written after it, so
	/// a following element is spaced from it.
	after_element: bool,
	buffer: String,
	/// When `true`, require an explicit [`MediaType::Text`] in accepts.
	/// When `false`, accept any text-based media type.
	plaintext_only: bool,
}

impl PlainTextRenderer {
	/// The elements that take lines of their own in plain text beyond the
	/// html block-level ones, laid out as rows wherever they render.
	const LINE_ELEMENTS: &[&str] = &["tr", "summary"];

	pub fn new() -> Self {
		Self {
			did_newline: true,
			after_element: false,
			buffer: String::new(),
			plaintext_only: false,
		}
	}

	/// Require an explicit [`MediaType::Text`] in accepts, rejecting other
	/// text-based media type like HTML or Markdown.
	pub fn plaintext_only(mut self) -> Self {
		self.plaintext_only = true;
		self
	}

	/// Consume the renderer and return the accumulated text.
	pub fn into_string(self) -> String { self.buffer }

	/// Whether `tag` takes lines of its own.
	fn breaks(tag: &str) -> bool {
		is_block_element(tag)
			|| Self::LINE_ELEMENTS
				.iter()
				.any(|line| line.eq_ignore_ascii_case(tag))
	}

	/// End the current line, unless it already ended.
	fn end_line(&mut self) {
		if !self.did_newline {
			self.buffer.push('\n');
			self.did_newline = true;
		}
		self.after_element = false;
	}
}

impl Default for PlainTextRenderer {
	fn default() -> Self { Self::new() }
}

impl NodeVisitor for PlainTextRenderer {
	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		if Self::breaks(view.element.tag()) {
			self.end_line();
		} else if self.after_element
			&& !self.buffer.ends_with(char::is_whitespace)
		{
			self.buffer.push(' ');
		}
		self.after_element = false;
		// plaintext ignores markup, but a control's typed value is text
		if let Some(value) = view.value {
			self.visit_value(cx, value);
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		match Self::breaks(element.tag())
			|| element.tag().eq_ignore_ascii_case("br")
		{
			true => self.end_line(),
			// an inline element flows, spaced only from an element after it
			false => self.after_element = !self.did_newline,
		}
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		self.buffer.push_str(&value.to_string());
		self.did_newline = false;
		self.after_element = false;
	}
}

impl NodeRenderer for PlainTextRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		let accepts = cx.accepts();
		if self.plaintext_only {
			cx.check_accepts(&[MediaType::Text])?;
		} else if !accepts.is_empty()
			&& !accepts.iter().any(|media_type| {
				media_type.is_wildcard() || media_type.is_text()
			}) {
			return Err(RenderError::AcceptMismatch {
				requested: accepts,
				available: vec![MediaType::Text],
			});
		}
		cx.walk(self);
		MediaBytes::new_string(
			MediaType::Text,
			core::mem::take(&mut self.buffer),
		)
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The plain text `bundle` renders as, answering a default request.
	fn text(bundle: impl Bundle) -> String {
		let mut world = World::new();
		let root = world.spawn(bundle).id();
		PlainTextRenderer::default()
			.render(&mut RenderContext::new(
				&mut world,
				root,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
	}

	/// The non-visual tags hold no prose: a `<script>` body and an `<svg>`'s
	/// `<text>` never reach the plain text.
	#[beet_core::test]
	fn skips_non_visual_tags() {
		text(rsx! {
			<div>
				<script>"alert(1)"</script>
				<svg viewBox="0 0 10 10"><text>"Picture"</text></svg>
				<p>"Visible"</p>
			</div>
		})
		.xpect_contains("Visible")
		.xnot()
		.xpect_contains("Picture")
		.xnot()
		.xpect_contains("alert");
	}

	/// One newline per block, inline elements flowing into their line.
	#[beet_core::test]
	fn one_line_per_block() {
		text(rsx! {
			<article>
				<h1>"Title"</h1>
				<p>"Some "<strong>"bold"</strong>" and "<a href="/x">"linked"</a>" prose."</p>
				<ul><li>"one"</li><li>"two"</li></ul>
			</article>
		})
			.xpect_eq("Title\nSome bold and linked prose.\none\ntwo\n");
	}

	/// A block starts its own line, a table row is a line of its cells, and
	/// two elements with nothing between them are spaced.
	#[beet_core::test]
	fn rows_and_items_stay_apart() {
		text(rsx! {
			<div>
				<nav><a href="/docs">"Docs"</a><a href="/blog">"Blog"</a></nav>
				<details><summary>"Docs ▾"</summary><ul><li>"Scenes"</li></ul></details>
				<table>
					<tr><th>"name"</th><th>"kind"</th></tr>
					<tr><td>"root"</td><td>"one of"</td></tr>
				</table>
			</div>
		})
		.xpect_eq("Docs Blog\nDocs ▾\nScenes\nname kind\nroot one of\n");
	}
}
