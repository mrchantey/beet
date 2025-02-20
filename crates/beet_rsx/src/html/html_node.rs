/// A trait for rendering a value to HTML
pub trait RenderHtml {
	/// Convert a value, usually [HtmlNode] to a string of HTML
	fn render(&self) -> String {
		let mut html = String::new();
		self.render_inner(&mut html);
		html
	}
	fn render_inner(&self, html: &mut String);
	/// Convert a value, usually [HtmlNode] to a string of HTML
	/// with indentation
	fn render_pretty(&self) -> String {
		let mut html = String::new();
		let mut indent = 0;
		self.render_pretty_inner(&mut html, &mut indent);
		html.pop(); // remove the last newline
		html
	}

	fn render_pretty_inner(&self, html: &mut String, indent: &mut usize);

	/// push a string with indentation, then add a newline
	fn push_pretty(html: &mut String, indent: &usize, str: &str) {
		for _ in 0..*indent {
			html.push('\t');
		}
		html.push_str(str);
		html.push('\n');
	}
}


impl RenderHtml for Vec<HtmlNode> {
	fn render_inner(&self, html: &mut String) {
		for node in self {
			node.render_inner(html);
		}
	}
	fn render_pretty_inner(&self, html: &mut String, indent: &mut usize) {
		for node in self {
			node.render_pretty_inner(html, indent);
		}
	}
}

/// Unlike RsxNode, this struct contains only real html nodes
#[derive(Debug, Clone)]
pub enum HtmlNode {
	Doctype,
	Comment(String),
	Text(String),
	Element(HtmlElementNode),
}

impl HtmlNode {
	/// recursively search for an html node with a matching id
	pub fn query_selector_attr(
		&mut self,
		key: &str,
		val: Option<&str>,
	) -> Option<&mut HtmlElementNode> {
		match self {
			HtmlNode::Element(e) => {
				if e.query_selector_attr(key, val) {
					return Some(e);
				}
				for child in &mut e.children {
					if let Some(node) = child.query_selector_attr(key, val) {
						return Some(node);
					}
				}
			}
			_ => {}
		}
		None
	}

	/// return self as an element if it matches the tag
	pub fn element_with_tag_owned(
		self,
		tag: &str,
	) -> Result<HtmlElementNode, Self> {
		match self {
			HtmlNode::Element(e) => {
				if e.tag == tag {
					return Ok(e);
				}
				Err(e.into())
			}
			_ => Err(self),
		}
	}
	/// return self as an element if it matches the tag
	pub fn element_with_tag(&self, tag: &str) -> Option<&HtmlElementNode> {
		match self {
			HtmlNode::Element(e) => {
				if e.tag == tag {
					return Some(e);
				}
			}
			_ => {}
		}
		None
	}
}

impl RenderHtml for HtmlNode {
	fn render_inner(&self, html: &mut String) {
		match self {
			HtmlNode::Doctype => html.push_str("<!DOCTYPE html>"),
			HtmlNode::Comment(val) => {
				html.push_str(&format!("<!-- {} -->", val))
			}
			HtmlNode::Text(val) => html.push_str(val),
			HtmlNode::Element(node) => node.render_inner(html),
		}
	}

	fn render_pretty_inner(&self, html: &mut String, indent: &mut usize) {
		match self {
			HtmlNode::Doctype => {
				Self::push_pretty(html, indent, "<!DOCTYPE html>")
			}
			HtmlNode::Comment(val) => {
				Self::push_pretty(html, indent, &format!("<!-- {} -->", val))
			}
			HtmlNode::Text(val) => Self::push_pretty(html, indent, val),
			HtmlNode::Element(node) => node.render_pretty_inner(html, indent),
		}
	}
}
#[derive(Debug, Clone)]
pub struct HtmlElementNode {
	pub tag: String,
	pub self_closing: bool,
	pub attributes: Vec<HtmlAttribute>,
	pub children: Vec<HtmlNode>,
}



impl Into<HtmlNode> for HtmlElementNode {
	fn into(self) -> HtmlNode { HtmlNode::Element(self) }
}

impl HtmlElementNode {
	pub fn inline_script(
		script: String,
		attributes: Vec<HtmlAttribute>,
	) -> Self {
		Self {
			tag: "script".to_string(),
			self_closing: false,
			attributes,
			children: vec![HtmlNode::Text(script)],
		}
	}


	/// returns true if any attribute matches the key and value
	pub fn query_selector_attr(
		&mut self,
		key: &str,
		val: Option<&str>,
	) -> bool {
		self.attributes
			.iter()
			.any(|a| a.key == key && a.value.as_deref() == val)
	}

	/// returns none if the attribute is not found or it has no value
	pub fn get_attribute_value(&self, key: &str) -> Option<&str> {
		for attr in &self.attributes {
			if attr.key == key {
				return attr.value.as_deref();
			}
		}
		None
	}
}

impl RenderHtml for HtmlElementNode {
	fn render_inner(&self, html: &mut String) {
		html.push_str(&format!("<{}", self.tag));
		for attribute in &self.attributes {
			attribute.render_inner(html);
		}

		if self.self_closing {
			html.push_str("/>");
			return;
		} else {
			html.push('>');
		}
		for child in &self.children {
			child.render_inner(html);
		}
		html.push_str(&format!("</{}>", self.tag));
	}
	fn render_pretty_inner(&self, html: &mut String, indent: &mut usize) {
		let mut open_tag = format!("<{}", self.tag);
		for attribute in &self.attributes {
			attribute.render_inner(&mut open_tag);
		}
		if self.self_closing {
			open_tag.push_str("/>");
			Self::push_pretty(html, indent, &open_tag);
			return;
		} else {
			open_tag.push('>');
		}

		Self::push_pretty(html, indent, &open_tag);
		*indent += 1;

		// unbreak consecutive text nodes
		let mut prev_text_node = false;
		for child in &self.children {
			if prev_text_node {
				// remove the newline and indent
				while html.ends_with('\n') || html.ends_with('\t') {
					html.pop();
				}
			}
			child.render_pretty_inner(html, indent);
			if let HtmlNode::Text(_) = child {
				prev_text_node = true;
			} else {
				prev_text_node = false;
			}
		}
		*indent -= 1;
		Self::push_pretty(html, indent, &format!("</{}>", self.tag));
	}
}
#[derive(Debug, Clone)]
pub struct HtmlAttribute {
	pub key: String,
	pub value: Option<String>,
}


impl RenderHtml for HtmlAttribute {
	fn render_inner(&self, html: &mut String) {
		html.push(' ');
		html.push_str(&self.key);
		if let Some(value) = &self.value {
			html.push_str("=\"");
			html.push_str(value);
			html.push_str("\"");
		}
	}

	fn render_pretty_inner(&self, _html: &mut String, _indent: &mut usize) {
		unimplemented!("attributes should be inline via render_inner")
	}
}


impl RenderHtml for Vec<HtmlAttribute> {
	fn render_inner(&self, html: &mut String) {
		for attr in self {
			attr.render_inner(html);
		}
	}
	fn render_pretty_inner(&self, html: &mut String, indent: &mut usize) {
		for attr in self {
			attr.render_pretty_inner(html, indent);
		}
	}
}





#[derive(Debug, Clone)]
pub struct HtmlConstants {
	/// the attribute for element ids, used for encoding the [TreePosition],
	pub tree_idx_key: &'static str,
	/// used for describing the location of rust blocks in text nodes,
	pub loc_map_key: &'static str,
	/// the global event handler for all events
	pub event_handler: &'static str,
	/// the global vec that stores prehydrated events
	pub event_store: &'static str,
}

impl Default for HtmlConstants {
	fn default() -> Self {
		Self {
			tree_idx_key: "data-beet-rsx-idx",
			loc_map_key: "data-beet-loc-map",
			event_handler: "_beet_event_handler",
			event_store: "_beet_event_store",
		}
	}
}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn pretty() {
		let val = "bar";
		let doc = rsx! {
			<html>
				<head>
					<title>Test "foo" {val}</title>
				</head>
				<body>
					<div foo="bar" bazz>
						<p>Test</p>
					</div>
				</body>
			</html>
		}
		.build_document()
		.unwrap();
		// println!("{}", doc.render_pretty());
		expect(doc.render_pretty()).to_be("<!DOCTYPE html>\n<html>\n\t<head>\n\t\t<title data-beet-rsx-idx=\"3\">\n\t\t\tTest \t\t\tfoo\t\t\tbar\n\t\t</title>\n\t</head>\n\t<body>\n\t\t<div foo=\"bar\" bazz>\n\t\t\t<p>\n\t\t\t\tTest\n\t\t\t</p>\n\t\t</div>\n\t</body>\n</html>");
	}
}
