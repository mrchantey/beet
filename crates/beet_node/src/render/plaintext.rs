use crate::prelude::*;
use beet_core::prelude::*;



/// Renders an entity tree as plain text, stripping all markup.
///
/// When `plaintext_only` is `true`, only [`MediaType::Text`] is accepted
/// in the `accepts` list. When `false` (the default), any text-based media
/// type is accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainTextRenderer {
	did_newline: bool,
	buffer: String,
	/// When `true`, require an explicit [`MediaType::Text`] in accepts.
	/// When `false`, accept any text-based media type.
	plaintext_only: bool,
	/// When `true`, decode HTML entities (eg `&amp;` → `&`) in text
	/// content via [`unescape_html_text`].
	unescape_html: bool,
}

impl PlainTextRenderer {
	pub fn new() -> Self {
		Self {
			did_newline: true,
			buffer: String::new(),
			plaintext_only: false,
			unescape_html: false,
		}
	}

	/// Require an explicit [`MediaType::Text`] in accepts, rejecting other
	/// text-based media type like HTML or Markdown.
	pub fn plaintext_only(mut self) -> Self {
		self.plaintext_only = true;
		self
	}

	/// Enable HTML entity unescaping in text output.
	///
	/// When enabled, entities like `&amp;`, `&lt;`, `&gt;` are decoded
	/// to their plain-text equivalents in rendered output.
	pub fn with_unescape_html(mut self) -> Self {
		self.unescape_html = true;
		self
	}

	/// Consume the renderer and return the accumulated text.
	pub fn into_string(self) -> String { self.buffer }
}

impl Default for PlainTextRenderer {
	fn default() -> Self { Self::new() }
}

impl NodeVisitor for PlainTextRenderer {
	fn visit_element(&mut self, _cx: &VisitContext, _view: ElementView) {
		// plaintext, ignore elements
	}

	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element) {
		// add a newline after every element, except if we just added one
		if !self.did_newline {
			self.buffer.push('\n');
			self.did_newline = true;
		}
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		let raw = value.to_string();
		if self.unescape_html {
			self.buffer.push_str(&unescape_html_text(&raw));
		} else {
			self.buffer.push_str(&raw);
		}
		self.did_newline = false;
	}
}

impl NodeRenderer for PlainTextRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<RenderOutput, RenderError> {
		if self.plaintext_only {
			cx.check_accepts(&[MediaType::Text])?;
		} else if !cx.accepts.is_empty()
			&& !cx.accepts.iter().any(|mt| mt.is_text())
		{
			return Err(RenderError::AcceptMismatch {
				requested: cx.accepts.clone(),
				available: vec![MediaType::Text],
			});
		}
		cx.walk(self);
		RenderOutput::media_string(
			MediaType::Text,
			std::mem::take(&mut self.buffer),
		)
		.xok()
	}
}
