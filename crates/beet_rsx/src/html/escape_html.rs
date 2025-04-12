use crate::prelude::*;

/// TODO this is incorrect, we need to escape RsxNode rust code
/// only.
pub struct EscapeHtml {
	/// Element tags, the children of which will not be escaped.
	/// Default: `["script", "style","code"]`
	pub ignored_tags: Vec<String>,
	pub escape_attributes: bool,
}


impl Default for EscapeHtml {
	fn default() -> Self {
		Self {
			ignored_tags: ["script", "style", "code"]
				.iter()
				.map(|s| s.to_string())
				.collect(),
			escape_attributes: false,
		}
	}
}

impl<T: AsMut<Vec<HtmlNode>>> Pipeline<T> for EscapeHtml {
	fn apply(self, mut value: T) -> T {
		self.escape_nodes(value.as_mut());
		value
	}
}


impl EscapeHtml {
	fn escape_nodes(&self, nodes: &mut Vec<HtmlNode>) {
		for node in nodes {
			match node {
				HtmlNode::Doctype => {}
				HtmlNode::Comment(inner) => *inner = escape(inner),
				HtmlNode::Text(text) => *text = escape(text),
				HtmlNode::Element(el) => {
					if self.escape_attributes {
						for attr in &mut el.attributes {
							self.escape_attribute(attr);
						}
					}
					// TODO use html_escape::encode_attribute etc
					if !self.ignored_tags.contains(&el.tag) {
						self.escape_nodes(&mut el.children);
					}
				}
			}
		}
	}
	fn escape_attribute(&self, attr: &mut HtmlAttribute) {
		attr.value = attr.value.as_ref().map(|v| escape(&v));
	}
}


fn escape(str: &str) -> String { html_escape::encode_safe(str).to_string() }

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[test]
	fn works() {
		expect(
			&vec![HtmlNode::Text("there's a snake in my boot".into())]
				.xpipe(RenderHtmlEscaped::default()),
		)
		.to_be("there&#x27;s a snake in my boot");
	}
}
