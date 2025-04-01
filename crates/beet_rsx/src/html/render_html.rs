use crate::prelude::*;
use anyhow::Result;

/// Convert [`HtmlNode`] structures into a string of html
/// - Panics if an element is self closing and has children
#[derive(Default)]
pub struct RenderHtml {
	pub pretty: bool,
}
impl RenderHtml {
	pub fn minimized() -> Self { Self { pretty: false } }
	pub fn pretty() -> Self { Self { pretty: true } }

	fn render(&self, nodes: &Vec<HtmlNode>) -> Result<String> {
		let mut html = String::new();
		for node in nodes {
			if self.pretty {
				render_node_pretty(&node, &mut html, &mut 0);
			} else {
				render_node(&node, &mut html);
			}
		}
		if self.pretty {
			// remove the last newline
			html.pop();
		}
		Ok(html)
	}
}

impl<T: AsRef<Vec<HtmlNode>>> RsxPipeline<T, Result<String>> for RenderHtml {
	fn apply(self, target: T) -> Result<String> {
		Ok(self.render(target.as_ref())?)
	}
}

impl RsxPipeline<HtmlDocument, Result<String>> for RenderHtml {
	fn apply(self, target: HtmlDocument) -> Result<String> {
		Ok(self.render(&target.into_nodes())?)
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



fn render_node(node: &HtmlNode, html: &mut String) {
	match node {
		HtmlNode::Doctype => html.push_str("<!DOCTYPE html>"),
		HtmlNode::Comment(val) => html.push_str(&format!("<!-- {} -->", val)),
		HtmlNode::Text(val) => html.push_str(val),
		HtmlNode::Element(el) => render_element(el, html),
	}
}

fn render_node_pretty(node: &HtmlNode, html: &mut String, indent: &mut usize) {
	match node {
		HtmlNode::Doctype => push_pretty(html, indent, "<!DOCTYPE html>"),
		HtmlNode::Comment(val) => {
			push_pretty(html, indent, &format!("<!-- {} -->", val))
		}
		HtmlNode::Text(val) => push_pretty(html, indent, val),
		HtmlNode::Element(el) => render_element_pretty(el, html, indent),
	}
}


fn render_element(el: &HtmlElementNode, html: &mut String) {
	el.assert_self_closing_no_children();
	html.push_str(&format!("<{}", el.tag));
	for attr in &el.attributes {
		render_attribute(attr, html);
	}

	if el.self_closing {
		html.push_str("/>");
		return;
	} else {
		html.push('>');
	}
	for child in &el.children {
		render_node(child, html);
	}
	html.push_str(&format!("</{}>", el.tag));
}
fn render_element_pretty(
	el: &HtmlElementNode,
	html: &mut String,
	indent: &mut usize,
) {
	el.assert_self_closing_no_children();
	let mut open_tag = format!("<{}", el.tag);
	for attr in &el.attributes {
		render_attribute(attr, &mut open_tag);
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
		render_node_pretty(child, html, indent);
		if let HtmlNode::Text(_) = child {
			prev_text_node = true;
		} else {
			prev_text_node = false;
		}
	}
	*indent -= 1;
	push_pretty(html, indent, &format!("</{}>", el.tag));
}

fn render_attribute(attr: &HtmlAttribute, html: &mut String) {
	html.push(' ');
	html.push_str(&attr.key);
	if let Some(value) = &attr.value {
		html.push_str("=\"");
		html.push_str(value);
		html.push_str("\"");
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
		.bpipe(RsxToHtmlDocument::default())
		.unwrap()
		.bpipe(RenderHtml::pretty())
		.unwrap();
		// println!("{}", doc.render_pretty());
		expect(doc).to_be("<!DOCTYPE html>\n<html>\n\t<head>\n\t\t<title data-beet-rsx-idx=\"4\">\n\t\t\tTest \t\t\tfoo\t\t\tbar\n\t\t</title>\n\t</head>\n\t<body>\n\t\t<div foo=\"bar\" bazz>\n\t\t\t<p>\n\t\t\t\tTest\n\t\t\t</p>\n\t\t</div>\n\t</body>\n</html>");
	}
}
