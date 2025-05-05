use crate::prelude::*;


/// Convenience pipeline to render html and escape it
#[derive(Default)]
pub struct RenderHtmlEscaped {
	pub render_html: RenderHtml,
	pub escape_html: EscapeHtml,
}

impl<T: AsMut<Vec<HtmlNode>> + AsRef<Vec<HtmlNode>>> Pipeline<T, String>
	for RenderHtmlEscaped
{
	fn apply(self, target: T) -> String {
		target.xpipe(self.escape_html).xpipe(self.render_html)
	}
}

impl Pipeline<HtmlDocument, String> for RenderHtmlEscaped {
	fn apply(self, target: HtmlDocument) -> String {
		target.into_nodes().xpipe(self)
	}
}



/// Convert [`HtmlNode`] structures into a string of html.
///
/// ## WARNING
/// This does not escape the html, only use this if you *really really*
/// know what you're doing.
#[derive(Default)]
pub struct RenderHtml;

impl<T: AsRef<Vec<HtmlNode>>> Pipeline<T, String> for RenderHtml {
	fn apply(self, target: T) -> String { self.render(target.as_ref()) }
}

impl Pipeline<HtmlDocument, String> for RenderHtml {
	fn apply(self, target: HtmlDocument) -> String {
		target.into_nodes().xpipe(self)
	}
}


impl RenderHtml {
	fn render(&self, nodes: &Vec<HtmlNode>) -> String {
		let mut html = String::new();
		for node in nodes {
			self.render_node(&node, &mut html);
		}
		html
	}



	fn render_node(&self, node: &HtmlNode, html: &mut String) {
		match node {
			HtmlNode::Doctype => html.push_str("<!DOCTYPE html>"),
			HtmlNode::Comment(val) => {
				html.push_str(&format!("<!-- {} -->", val))
			}
			HtmlNode::Text(val) => html.push_str(&val),
			HtmlNode::Element(el) => self.render_element(el, html),
		}
	}

	fn render_element(&self, el: &HtmlElementNode, html: &mut String) {
		el.assert_self_closing_no_children();
		html.push_str(&format!("<{}", el.tag));
		for attr in &el.attributes {
			self.render_attribute(attr, html);
		}

		if el.self_closing {
			html.push_str("/>");
			return;
		} else {
			html.push('>');
		}
		for child in &el.children {
			self.render_node(child, html);
		}
		html.push_str(&format!("</{}>", el.tag));
	}

	fn render_attribute(&self, attr: &HtmlAttribute, html: &mut String) {
		html.push(' ');
		html.push_str(&attr.key);
		if let Some(value) = &attr.value {
			html.push_str("=\"");
			html.push_str(value);
			html.push_str("\"");
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
						<p>Bazz</p>
					</div>
				</body>
			</html>
		}
		.xpipe(RsxToHtmlDocument::default())
		.unwrap()
		.xpipe(RenderHtml::default());
		// println!("{}", doc.render_pretty());
		expect(doc).to_be(
			"<!DOCTYPE html><html><head><title data-beet-rsx-idx=\"4\">Test foobar</title></head><body><div foo=\"bar\" bazz><p>Bazz</p></div></body></html>",
		);
	}
}
