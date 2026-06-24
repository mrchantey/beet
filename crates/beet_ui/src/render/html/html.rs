use crate::prelude::*;
use alloc::borrow::Cow;
use beet_core::prelude::*;

/// Renders an entity tree back to an HTML string via [`NodeVisitor`].
///
/// Supports pretty-printing with configurable indentation,
/// void elements, and optional expression rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlRenderer {
	buffer: String,
	/// HTML fragments to hoist into `<head>` (emitted before `</head>`),
	/// contributed by any render feature via [`hoist_into_head`](Self::hoist_into_head).
	/// Drained once at the head close, or post-walk for a fragment with no
	/// `<head>` (the emptied vec gives once-only semantics). The reactive renderer
	/// is the first contributor; a future build-time `bx:hoist` directive is
	/// intended to feed this same collection.
	head_hoist: Vec<String>,
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
	/// When `true`, escape special characters in text content and
	/// attribute values using [`escape_html_text`] and
	/// [`escape_html_attribute`] respectively.
	escape_html: bool,
	/// When `true` (the default), authoring comments (`<!-- note -->`) are
	/// stripped so they never leak into rendered output. Functional `bx`-prefixed
	/// anchors (`<!--bx-ref-->`) are always kept regardless.
	strip_comments: bool,
	/// Raw text elements whose children should not be HTML-escaped,
	/// per the HTML spec, ie `<script>`, `<style>`.
	raw_text_elements: Vec<Cow<'static, str>>,
	/// Tracks whether we are currently inside a raw text element.
	in_raw_text_element: bool,
	/// When set, the renderer also emits the thin-client reactivity wire format
	/// (see [`reactive_html_render`](super::reactive_html_render)); the resolved
	/// text is unchanged, so a non-reactive page is byte-identical.
	#[cfg(all(feature = "bsx", feature = "json"))]
	reactive: Option<super::reactive_html_render::ReactiveHtmlRender>,
}

/// Indentation style for pretty-printing.
#[derive(Debug, Clone, PartialEq, Eq)]
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
			head_hoist: Vec::new(),
			indent: None,
			render_expressions: false,
			void_elements: default_void_elements(),
			current_depth: 0,
			escape_html: true,
			strip_comments: true,
			raw_text_elements: default_raw_text_elements(),
			in_raw_text_element: false,
			#[cfg(all(feature = "bsx", feature = "json"))]
			reactive: None,
		}
	}

	/// Enable pretty-printing with the default indentation (one tab).
	pub fn pretty(mut self) -> Self {
		self.indent = Some(Indent::default());
		self
	}

	/// Emit the thin-client reactivity wire format alongside the resolved HTML:
	/// `data-bx-doc`/`data-bx-attr`/`bx:<event>` annotations, `<!--bx-ref-->` text
	/// anchors, the document/verb blobs, and the runtime script (in `<head>`). In
	/// the default [`InsertReactive::Auto`] mode the resolved text is unchanged and
	/// a page with no bindings renders identically to the static output.
	#[cfg(all(feature = "bsx", feature = "json"))]
	pub fn reactive(self) -> Self {
		self.reactive_with(InsertReactive::default(), false)
	}

	/// [`reactive`](Self::reactive) with explicit config: when to inject the
	/// runtime ([`InsertReactive`]) and whether to inline [`REACTIVITY_JS`] rather
	/// than reference it by URL.
	#[cfg(all(feature = "bsx", feature = "json"))]
	pub fn reactive_with(
		mut self,
		insert_reactive: InsertReactive,
		inline_js_runtime: bool,
	) -> Self {
		self.reactive =
			Some(super::reactive_html_render::ReactiveHtmlRender::new(
				insert_reactive,
				inline_js_runtime,
			));
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

	/// Keep authoring comments (`<!-- note -->`) in the output rather than
	/// stripping them (the default). Functional `bx`-prefixed anchors are emitted
	/// either way.
	pub fn with_comments(mut self) -> Self {
		self.strip_comments = false;
		self
	}

	/// Enable HTML entity escaping for text content and attribute values.
	///
	/// When enabled, characters like `<`, `>`, `&`, `"` are replaced
	/// with their HTML entity equivalents in the output.
	pub fn with_escape_html(mut self) -> Self {
		self.escape_html = true;
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

	/// Contribute an HTML fragment to hoist into `<head>` (emitted before
	/// `</head>`, or post-walk for a fragment with no `<head>`). Any feature can
	/// push here; the reactive renderer is the first contributor.
	pub fn hoist_into_head(&mut self, fragment: impl Into<String>) {
		self.head_hoist.push(fragment.into());
	}

	/// Consume the renderer and return the accumulated HTML string.
	pub fn into_string(self) -> String { self.buffer }

	/// Borrow the accumulated HTML string.
	pub fn as_str(&self) -> &str { &self.buffer }

	fn is_void_element(&self, name: &str) -> bool {
		let lower = name.to_ascii_lowercase();
		self.void_elements.iter().any(|el| el.as_ref() == lower)
	}

	fn is_raw_text_element(&self, name: &str) -> bool {
		self.raw_text_elements.iter().any(|el| el.as_ref() == name)
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

	/// Drain the hoisted head fragments, concatenated in push order, or `None`
	/// when empty. Draining empties the collection, so the post-walk fallback
	/// fires only when the in-walk `<head>` close did not (once-only without a
	/// flag).
	fn take_head_hoist(&mut self) -> Option<String> {
		if self.head_hoist.is_empty() {
			return None;
		}
		Some(core::mem::take(&mut self.head_hoist).concat())
	}
}

impl NodeVisitor for HtmlRenderer {
	// html renderer visits every node
	fn skip_node(&mut self, _node: &NodeView) -> bool { false }

	fn visit_doctype(&mut self, _cx: &VisitContext, doctype: &Doctype) {
		self.write_indent();
		// the stored value is the doctype's value with the keyword stripped,
		// ie `"html"` (see [`Doctype::new`])
		self.buffer.push_str("<!DOCTYPE ");
		self.buffer.push_str(doctype);
		self.buffer.push('>');
		self.write_newline();
	}

	fn visit_comment(&mut self, _cx: &VisitContext, comment: &Comment) {
		// Functional `bx`-prefixed anchors (eg `<!--bx-ref-->`, `<!--bx-end-->`)
		// always render. Authoring comments are stripped by default (the framework
		// emits anchors without a leading space, so a space-led `<!-- note -->` is
		// authoring), unless `with_comments` opted to keep them.
		if self.strip_comments && !comment.starts_with("bx") {
			return;
		}
		self.write_indent();
		self.buffer.push_str("<!--");
		self.buffer.push_str(comment);
		self.buffer.push_str("-->");
		self.write_newline();
	}

	fn visit_element(&mut self, _cx: &VisitContext, view: ElementView) {
		self.write_indent();
		self.buffer.push('<');
		self.buffer.push_str(view.tag());

		// reactive mode: emit `data-bx-doc`, `bx:<event>`, and `data-bx-attr-*`
		// for this element (computed first so the `reactive` borrow ends before
		// the buffer writes).
		#[cfg(all(feature = "bsx", feature = "json"))]
		{
			let fragments = self
				.reactive
				.as_mut()
				.map(|reactive| {
					let mut fragments = Vec::new();
					if let Some(id) = reactive.enter_element(_cx.entity) {
						fragments.push(format!("data-bx-doc=\"d{id}\""));
					}
					for (event, call) in reactive.events(_cx.entity) {
						fragments.push(format!(
							"bx:{event}=\"{}\"",
							escape_html_attribute(call)
						));
					}
					for attr in &view.attributes {
						if let Some(path) = reactive.attr_path(attr.entity) {
							fragments.push(format!(
								"data-bx-attr-{}=\"{}\"",
								attr.attribute.as_str(),
								escape_html_attribute(path)
							));
						}
					}
					fragments
				})
				.unwrap_or_default();
			for fragment in fragments {
				self.buffer.push(' ');
				self.buffer.push_str(&fragment);
			}
		}

		for attr in &view.attributes {
			// the `class` attribute is merged with the `Classes` component and
			// emitted once below, so skip the raw attribute here
			if attr.attribute.as_str() == "class" {
				continue;
			}
			self.buffer.push(' ');
			self.buffer.push_str(attr.attribute.as_str());
			match attr.value {
				Value::Null => {
					// boolean attribute, no value
				}
				_ => {
					self.buffer.push_str("=\"");
					let raw = attr.value.to_string();
					if self.escape_html {
						self.buffer.push_str(&escape_html_attribute(&raw));
					} else {
						self.buffer.push_str(&raw);
					}
					self.buffer.push('"');
				}
			}
		}

		// merge the `Classes` component with any `class` attribute into a single
		// deterministic `class="…"`, so widget-emitted semantic classes reach the
		// rendered HTML (and the stylesheet that targets them)
		let mut classes: Vec<SmolStr> = view.iter_classes().collect();
		if !classes.is_empty() {
			classes.sort();
			classes.dedup();
			let joined = classes.join(" ");
			self.buffer.push_str(" class=\"");
			if self.escape_html {
				self.buffer.push_str(&escape_html_attribute(&joined));
			} else {
				self.buffer.push_str(&joined);
			}
			self.buffer.push('"');
		}

		let is_void = self.is_void_element(view.tag());
		let is_raw = self.is_raw_text_element(view.tag());

		if is_void {
			self.buffer.push_str(" />");
		} else {
			self.buffer.push('>');
		}

		if is_raw {
			self.in_raw_text_element = true;
		}

		if self.is_pretty() {
			self.buffer.push('\n');
			self.current_depth += 1;
		}
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		// reactive mode: pop this element's document scope (before the void
		// early-return, so the stack balances with `visit_element`).
		#[cfg(all(feature = "bsx", feature = "json"))]
		if let Some(reactive) = self.reactive.as_mut() {
			reactive.leave_element();
		}
		let is_void = self.is_void_element(element.tag());
		if is_void {
			return;
		}

		if self.is_raw_text_element(element.tag()) {
			self.in_raw_text_element = false;
		}

		// inject any hoisted head fragments inside `<head>`, before its close tag.
		// `<head>` is the single injection point; a fragment with no head falls
		// back to a post-walk append in `render`. Any feature can contribute (eg
		// the reactive renderer's blob + runtime); a future build-time `bx:hoist`
		// directive is intended to feed the same collection.
		if element.tag() == "head" {
			if let Some(hoisted) = self.take_head_hoist() {
				self.write_indent();
				self.buffer.push_str(&hoisted);
				self.write_newline();
			}
		}

		if self.is_pretty() {
			self.current_depth = self.current_depth.saturating_sub(1);
			self.write_indent();
		}

		self.buffer.push_str("</");
		self.buffer.push_str(element.tag());
		self.buffer.push('>');
		self.write_newline();
	}

	fn visit_value(&mut self, _cx: &VisitContext, value: &Value) {
		// reactive mode: a bound text node is wrapped in `<!--bx-ref-->` anchors so
		// the runtime can patch just this run (computed first to end the borrow).
		#[cfg(all(feature = "bsx", feature = "json"))]
		let bx_ref = self
			.reactive
			.as_ref()
			.and_then(|reactive| reactive.text_path(_cx.entity))
			.map(|path| escape_html_attribute(path));
		if self.is_pretty() {
			self.write_indent();
		}
		#[cfg(all(feature = "bsx", feature = "json"))]
		if let Some(path) = &bx_ref {
			self.buffer.push_str("<!--bx-ref=\"");
			self.buffer.push_str(path);
			self.buffer.push_str("\"-->");
		}
		let raw = value.to_string();
		if self.escape_html && !self.in_raw_text_element {
			self.buffer.push_str(&escape_html_text(&raw));
		} else {
			self.buffer.push_str(&raw);
		}
		#[cfg(all(feature = "bsx", feature = "json"))]
		if bx_ref.is_some() {
			self.buffer.push_str("<!--bx-end-->");
		}
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

impl NodeRenderer for HtmlRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[MediaType::Html])?;
		// collect the reactivity annotations once (the only world-reading pass)
		// before the walk, then register them as one head-hoist contributor so the
		// fragment is known when `<head>` closes.
		#[cfg(all(feature = "bsx", feature = "json"))]
		if let Some(reactive) = self.reactive.as_mut() {
			reactive.collect(cx.world, cx.entity);
			if let Some(fragment) = reactive.into_head_fragment() {
				self.hoist_into_head(fragment);
			}
		}
		cx.walk(self);
		// fallback: a fragment with no `<head>` never triggered the in-walk drain,
		// so emit any remaining hoisted fragments now (a no-op when the walk did).
		if let Some(hoisted) = self.take_head_hoist() {
			self.buffer.push_str(&hoisted);
		}
		MediaBytes::new_string(
			MediaType::Html,
			core::mem::take(&mut self.buffer),
		)
		.xok()
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

/// Raw text elements whose children must not be HTML-escaped,
/// per the HTML spec.
fn default_raw_text_elements() -> Vec<Cow<'static, str>> {
	vec!["script".into(), "style".into()]
}

#[cfg(test)]
mod test {
	use super::*;

	#[cfg(feature = "bsx")]
	/// Parse HTML (via the BSX parser's HTML mode) into a world, returning the
	/// root entity ready to render.
	fn parse_html(html: &str) -> (World, Entity) {
		let mut world = ui_world();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_html(html);
		BsxParser::html()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		(world, entity)
	}

	#[cfg(feature = "bsx")]
	/// Parse HTML then render it back with the given renderer.
	fn roundtrip_with(html: &str, mut renderer: HtmlRenderer) -> String {
		let (mut world, entity) = parse_html(html);
		renderer
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
	}

	#[cfg(feature = "bsx")]
	/// Parse HTML then render it back.
	fn roundtrip(html: &str) -> String {
		roundtrip_with(html, HtmlRenderer::new())
	}

	#[cfg(feature = "bsx")]
	/// Parse then render with expression support.
	fn roundtrip_expressions(html: &str) -> String {
		roundtrip_with(html, HtmlRenderer::new().with_expressions())
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_simple_element() {
		roundtrip("<div>hello</div>").xpect_eq("<div>hello</div>".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_nested_elements() {
		roundtrip("<div><span>inner</span></div>")
			.xpect_eq("<div><span>inner</span></div>".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_void_element() {
		roundtrip("<div><br>text</div>")
			.xpect_eq("<div><br />text</div>".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn strips_plain_comment() {
		// authoring comments are stripped so they never leak into rendered HTML
		roundtrip("<!-- hello -->").xpect_eq(String::new());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn keeps_bx_comment() {
		// functional `bx`-prefixed anchors survive (eg the reactive ref/end markers)
		roundtrip("<!--bx-ref-->").xpect_eq("<!--bx-ref-->".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_doctype() {
		roundtrip("<!DOCTYPE html>").xpect_eq("<!DOCTYPE html>".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_text_only() {
		roundtrip("hello world").xpect_eq("hello world".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_expression() {
		roundtrip_expressions("<p>{name}</p>")
			.xpect_eq("<p>{name}</p>".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_attributes() {
		roundtrip("<div class=\"foo\" id=\"bar\"></div>")
			.xpect_contains("class=\"foo\"")
			.xpect_contains("id=\"bar\"");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn render_self_closing() {
		roundtrip("<img src=\"foo.png\" />")
			.xpect_contains("<img")
			.xpect_contains("/>");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn non_reactive_caller_hoists_into_head() {
		// a plain (non-reactive) contributor pushes a <meta> that must land inside
		// <head>, before </head>, with no reactive feature involved.
		let mut renderer = HtmlRenderer::new();
		renderer.hoist_into_head("<meta name=\"x\" content=\"y\">");
		roundtrip_with(
			"<html><head><title>t</title></head><body></body></html>",
			renderer,
		)
		.split("</head>")
		.next()
		.unwrap()
		.to_string()
		.xpect_contains("<meta name=\"x\" content=\"y\">");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn head_hoist_falls_back_with_no_head() {
		// a fragment with no <head> never hits the in-walk drain, so the hoisted
		// element is appended post-walk via the same collection.
		let mut renderer = HtmlRenderer::new();
		renderer.hoist_into_head("<meta name=\"x\">");
		roundtrip_with("<div>hi</div>", renderer)
			.xpect_contains("<meta name=\"x\">");
	}

	#[beet_core::test]
	fn renders_classes_component_as_class_attribute() {
		// the `Classes` component (no `class` attribute) reaches the HTML output,
		// sorted and merged so the stylesheet's selectors match
		let mut world = World::new();
		let root = world
			.spawn((Element::new("button"), Classes::new(["btn-error", "btn"])))
			.id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("class=\"btn btn-error\"");
	}

	#[beet_core::test]
	fn escape_text_content() {
		// The HTML parser stores text verbatim (no entity decoding),
		// so we manually build an entity tree with a raw `&` in the
		// Value to verify the renderer escapes it.
		let mut world = World::new();
		let root = world.spawn(Element::new("p")).id();
		let text = world.spawn(Value::new("a & b")).id();
		world.entity_mut(root).add_children(&[text]);

		HtmlRenderer::new()
			.with_escape_html()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("a &amp; b");
	}

	#[beet_core::test]
	fn no_escape_script_content() {
		// Text inside <script> should not be HTML-escaped.
		let mut world = World::new();
		let root = world.spawn(Element::new("script")).id();
		let text = world.spawn(Value::new("let x = 1 < 2;")).id();
		world.entity_mut(root).add_children(&[text]);

		HtmlRenderer::new()
			.with_escape_html()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("let x = 1 < 2;")
			.xnot()
			.xpect_contains("&lt;");
	}

	#[beet_core::test]
	fn no_escape_style_content() {
		// Text inside <style> should not be HTML-escaped.
		let mut world = World::new();
		let root = world.spawn(Element::new("style")).id();
		let text = world
			.spawn(Value::new("body { font-family: 'a' & 'b'; }"))
			.id();
		world.entity_mut(root).add_children(&[text]);

		HtmlRenderer::new()
			.with_escape_html()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("body { font-family: 'a' & 'b'; }")
			.xnot()
			.xpect_contains("&amp;");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn roundtrip_script_raw_text() {
		// Parsing and re-rendering script content should preserve raw text.
		roundtrip("<script>let x = 1 < 2;</script>")
			.xpect_eq("<script>let x = 1 < 2;</script>".to_string());
	}

	#[beet_core::test]
	fn escape_attribute_value() {
		// Manually build an element with a raw `&` in an attribute
		// value to verify the renderer escapes it in attribute context.
		// Attributes use the `AttributeOf` relationship.
		let mut world = World::new();
		let root = world.spawn(Element::new("a")).id();
		let text = world.spawn(Value::new("link")).id();
		world.entity_mut(root).add_children(&[text]);
		// Spawn attribute with raw ampersand via the relationship.
		world.spawn((
			Attribute::new("href"),
			Value::new("a&b"),
			AttributeOf::new(root),
		));

		HtmlRenderer::new()
			.with_escape_html()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains("href=\"a&amp;b\"");
	}
}
