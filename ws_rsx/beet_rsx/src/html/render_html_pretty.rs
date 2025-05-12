use crate::prelude::*;

/// Render with indentation, this breaks hydration
/// so should only be used for debugging
/// - Panics if an element is self closing and has children
#[derive(Default)]
pub struct RenderHtmlPretty;

impl<T: AsRef<Vec<HtmlNode>>> Pipeline<T, String> for RenderHtmlPretty {
	fn apply(self, target: T) -> String { self.render(target.as_ref()) }
}

impl Pipeline<HtmlDocument, String> for RenderHtmlPretty {
	fn apply(self, target: HtmlDocument) -> String {
		self.render(&target.into_nodes())
	}
}


impl RenderHtmlPretty {
	fn render(&self, nodes: &Vec<HtmlNode>) -> String {
		let mut html = String::new();
		for node in nodes {
			self.render_node_pretty(&node, &mut html, &mut 0);
		}
		// remove the last newline
		html.pop();
		html
	}

	fn render_node_pretty(
		&self,
		node: &HtmlNode,
		html: &mut String,
		indent: &mut usize,
	) {
		match node {
			HtmlNode::Doctype => push_pretty(html, indent, "<!DOCTYPE html>"),
			HtmlNode::Comment(val) => {
				push_pretty(html, indent, &format!("<!-- {} -->", val))
			}
			HtmlNode::Text(val) => push_pretty(html, indent, &val),
			HtmlNode::Element(el) => {
				self.render_element_pretty(el, html, indent)
			}
		}
	}
	fn render_element_pretty(
		&self,
		el: &HtmlElementNode,
		html: &mut String,
		indent: &mut usize,
	) {
		el.assert_self_closing_no_children();
		let mut open_tag = format!("<{}", el.tag);
		for attr in &el.attributes {
			self.render_attribute(attr, &mut open_tag);
		}
		if el.self_closing {
			open_tag.push_str("/>");
			push_pretty(html, indent, &open_tag);
			return;
		} else {
			open_tag.push('>');
		}

		push_pretty(html, indent, &open_tag);
		*indent += 1;

		// unbreak consecutive text nodes
		let mut prev_text_node = false;
		for child in &el.children {
			if prev_text_node {
				// remove the newline and indent
				while html.ends_with('\n') || html.ends_with('\t') {
					html.pop();
				}
			}
			self.render_node_pretty(child, html, indent);
			if let HtmlNode::Text(_) = child {
				prev_text_node = true;
			} else {
				prev_text_node = false;
			}
		}
		*indent -= 1;
		push_pretty(html, indent, &format!("</{}>", el.tag));
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


/// push a string with indentation, then add a newline
fn push_pretty(html: &mut String, indent: &usize, str: &str) {
	for _ in 0..*indent {
		html.push('\t');
	}
	html.push_str(str);
	html.push('\n');
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
		.xpipe(RenderHtmlPretty::default());
		// println!("{}", doc.render_pretty());
		expect(doc).to_be(
			r#"<!DOCTYPE html>
<html>
	<head>
		<title data-beet-rsx-idx="4">
			Test 			foo			bar
		</title>
	</head>
	<body>
		<div foo="bar" bazz>
			<p>
				Bazz
			</p>
		</div>
	</body>
</html>"#,
		);
	}
}
