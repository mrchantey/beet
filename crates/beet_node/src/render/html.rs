use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Renders an entity tree back to an HTML string via [`NodeVisitor`].
///
/// Supports pretty-printing with configurable indentation,
/// void elements, and optional expression rendering.
pub struct HtmlRenderer {
	buffer: String,
	/// If `Some`, creates newlines after open/close tags
	/// and indents children with the provided indentation.
	indent: Option<Indent>,
	/// Render [`Expression`] values verbatim as `{expr}` in output.
	render_expressions: bool,
	/// Elements without a closing tag, whose children
	/// will be popped out to trailing siblings.
	void_elements: Vec<Cow<'static, str>>,
	/// Current indentation depth, used with `indent`.
	current_depth: usize,
}

/// Indentation style for pretty-printing.
pub enum Indent {
	Tabs(u8),
	Spaces(u8),
}

impl Default for Indent {
	fn default() -> Self { Self::Tabs(1) }
}

impl Indent {
	/// Produce the indentation string for a single level.
	fn unit(&self) -> String {
		match self {
			Indent::Tabs(count) => "\t".repeat(*count as usize),
			Indent::Spaces(count) => " ".repeat(*count as usize),
		}
	}
}

impl Default for HtmlRenderer {
	fn default() -> Self { Self::new() }
}

impl HtmlRenderer {
	pub fn new() -> Self {
		Self {
			buffer: String::new(),
			indent: None,
			render_expressions: false,
			void_elements: default_void_elements(),
			current_depth: 0,
		}
	}

	/// Enable pretty-printing with the default indentation (one tab).
	pub fn pretty(mut self) -> Self {
		self.indent = Some(Indent::default());
		self
	}

	/// Enable pretty-printing with custom indentation.
	pub fn with_indent(mut self, indent: Indent) -> Self {
		self.indent = Some(indent);
		self
	}

	/// Enable rendering of [`Expression`] nodes as `{expr}`.
	pub fn with_expressions(mut self) -> Self {
		self.render_expressions = true;
		self
	}

	/// Override the set of void elements.
	pub fn with_void_elements(
		mut self,
		elements: Vec<Cow<'static, str>>,
	) -> Self {
		self.void_elements = elements;
		self
	}

	/// Consume the renderer and return the accumulated HTML string.
	pub fn into_string(self) -> String { self.buffer }

	/// Borrow the accumulated HTML string.
	pub fn as_str(&self) -> &str { &self.buffer }

	fn is_void_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.void_elements.iter().any(|el| el.as_ref() == lower)
	}

	fn is_pretty(&self) -> bool { self.indent.is_some() }

	/// Write indentation for the current depth (only in pretty mode).
	fn write_indent(&mut self) {
		if let Some(ref indent) = self.indent {
			let unit = indent.unit();
			for _ in 0..self.current_depth {
				self.buffer.push_str(&unit);
			}
		}
	}

	/// Write a newline (only in pretty mode).
	fn write_newline(&mut self) {
		if self.is_pretty() {
			self.buffer.push('\n');
		}
	}
}


impl NodeVisitor for HtmlRenderer {
	fn visit_doctype(&mut self, _cx: &VisitContext, doctype: &Doctype) {
		self.write_indent();
		// The stored value is the raw doctype text, ie `"DOCTYPE html"`.
		// We wrap it in `<!` and `>`.
		self.buffer.push_str("<!");
		self.buffer.push_str(doctype);
		self.buffer.push('>');
		self.write_newline();
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		self.write_indent();
		self.buffer.push_str("<!--");
		self.buffer.push_str(comment);
		self.buffer.push_str("-->");
		self.write_newline();
	}

	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		element: &Element,
		attrs: Vec<(Entity, &Attribute, &Value)>,
	) {
		self.write_indent();
		self.buffer.push('<');
		self.buffer.push_str(element.name());

		for (_entity, attr, value) in &attrs {
			self.buffer.push(' ');
			self.buffer.push_str(attr);
			match value {
				Value::Null => {
					// boolean attribute, no value
				}
				_ => {
					self.buffer.push_str("=\"");
					self.buffer.push_str(&value.to_string());
					self.buffer.push('"');
				}
			}
		}

		let is_void = self.is_void_element(element.name());
		if is_void {
			self.buffer.push_str(" />");
		} else {
			self.buffer.push('>');
		}

		if self.is_pretty() {
			self.buffer.push('\n');
			self.current_depth += 1;
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let is_void = self.is_void_element(element.name());
		if is_void {
			return;
		}

		if self.is_pretty() {
			self.current_depth = self.current_depth.saturating_sub(1);
			self.write_indent();
		}

		self.buffer.push_str("</");
		self.buffer.push_str(element.name());
		self.buffer.push('>');
		self.write_newline();
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		if self.is_pretty() {
			self.write_indent();
		}
		self.buffer.push_str(&value.to_string());
		if self.is_pretty() {
			self.buffer.push('\n');
		}
	}

	fn visit_expression(
		&mut self,
		_cx: &VisitContext,
		expression: &Expression,
	) {
		if self.render_expressions {
			if self.is_pretty() {
				self.write_indent();
			}
			self.buffer.push('{');
			self.buffer.push_str(&expression.0);
			self.buffer.push('}');
			if self.is_pretty() {
				self.buffer.push('\n');
			}
		}
	}
}


fn default_void_elements() -> Vec<Cow<'static, str>> {
	vec![
		"area".into(),
		"base".into(),
		"br".into(),
		"col".into(),
		"embed".into(),
		"hr".into(),
		"img".into(),
		"input".into(),
		"link".into(),
		"meta".into(),
		"param".into(),
		"source".into(),
		"track".into(),
		"wbr".into(),
	]
}


#[cfg(test)]
mod test {
	use super::*;

	/// Helper to parse HTML then render it back via [`HtmlRenderer`].
	async fn roundtrip(html: &[u8]) -> String {
		let mut world_handle = AsyncPlugin::world();
		let html_owned = html.to_vec();
		world_handle
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let id = entity.id();
				entity
					.world()
					.with_then(move |world| {
						HtmlParser::new()
							.parse(world, id, html_owned, None)
							.unwrap();
						let mut renderer = Some(HtmlRenderer::new());
						world
							.run_system_once(move |walker: NodeWalker| {
								let mut r = renderer.take().unwrap();
								walker.walk(&mut r, id);
								r.into_string()
							})
							.unwrap()
					})
					.await
			})
			.await
	}

	/// Helper to parse then render with expression support.
	async fn roundtrip_expressions(html: &[u8]) -> String {
		let mut world_handle = AsyncPlugin::world();
		let html_owned = html.to_vec();
		world_handle
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let id = entity.id();
				entity
					.world()
					.with_then(move |world| {
						HtmlParser::with_expressions()
							.parse(world, id, html_owned, None)
							.unwrap();
						let mut renderer =
							Some(HtmlRenderer::new().with_expressions());
						world
							.run_system_once(move |walker: NodeWalker| {
								let mut r = renderer.take().unwrap();
								walker.walk(&mut r, id);
								r.into_string()
							})
							.unwrap()
					})
					.await
			})
			.await
	}

	#[beet_core::test]
	async fn render_simple_element() {
		roundtrip(b"<div>hello</div>")
			.await
			.xpect_eq("<div>hello</div>".to_string());
	}

	#[beet_core::test]
	async fn render_nested_elements() {
		roundtrip(b"<div><span>inner</span></div>")
			.await
			.xpect_eq("<div><span>inner</span></div>".to_string());
	}

	#[beet_core::test]
	async fn render_void_element() {
		roundtrip(b"<div><br>text</div>")
			.await
			.xpect_eq("<div><br />text</div>".to_string());
	}

	#[beet_core::test]
	async fn render_comment() {
		roundtrip(b"<!-- hello -->")
			.await
			.xpect_eq("<!-- hello -->".to_string());
	}

	#[beet_core::test]
	async fn render_text_only() {
		roundtrip(b"hello world")
			.await
			.xpect_eq("hello world".to_string());
	}

	#[beet_core::test]
	async fn render_expression() {
		roundtrip_expressions(b"<p>{name}</p>")
			.await
			.xpect_eq("<p>{name}</p>".to_string());
	}

	#[beet_core::test]
	async fn render_attributes() {
		roundtrip(b"<div class=\"foo\" id=\"bar\"></div>")
			.await
			.xpect_contains("class=\"foo\"")
			.xpect_contains("id=\"bar\"");
	}

	#[beet_core::test]
	async fn render_self_closing() {
		roundtrip(b"<img src=\"foo.png\" />")
			.await
			.xpect_contains("<img")
			.xpect_contains("/>");
	}
}
