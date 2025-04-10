use crate::prelude::*;

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



/// escape html characters unless `no_escape` is set
fn escape(str: &str) -> String {
	str.replace("&", "&amp;")
		.replace("<", "&lt;")
		.replace(">", "&gt;")
		.replace("\"", "&quot;")
		.replace("'", "&apos;")
}

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
		.to_be("there&apos;s a snake in my boot");
	}
}
