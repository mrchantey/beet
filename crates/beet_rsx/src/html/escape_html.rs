use crate::prelude::*;

/// Escapes HTML content to prevent XSS attacks. This does not escape
/// ignored tags, which are `script`, `style`, and `code` by default,
/// so do not include any user input in these tags.
pub struct EscapeHtml {
	/// Element tags, the children of which will not be escaped.
	/// Default: `["script", "style","code"]`
	pub ignored_tags: Vec<String>,
}


impl Default for EscapeHtml {
	fn default() -> Self {
		Self {
			ignored_tags: ["script", "style", "code"]
				.iter()
				.map(|s| s.to_string())
				.collect(),
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
	fn escape_nodes(&self, _nodes: &mut Vec<HtmlNode>) {
		// TODO rsx node escaping, only of mutable content

		// for node in nodes {
		// 	match node {
		// 		HtmlNode::Doctype => {}
		// 		HtmlNode::Comment(inner) => {
		// 			*inner = html_escape::encode_text(inner).to_string()
		// 		}
		// 		HtmlNode::Text(text) => {
		// 			*text = html_escape::encode_text(text).to_string()
		// 		}
		// 		HtmlNode::Element(el) => {
		// 			for value in &mut el
		// 				.attributes
		// 				.iter_mut()
		// 				.filter_map(|a| a.value.as_mut())
		// 			{
		// 				*value =
		// 					html_escape::encode_double_quoted_attribute(value)
		// 						.to_string();
		// 			}
		// 			if !self.ignored_tags.contains(&el.tag) {
		// 				self.escape_nodes(&mut el.children);
		// 			}
		// 		}
		// 	}
		// }
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[test]
	#[ignore = "todo rsx escaping"]
	fn works() {
		expect(
			&vec![HtmlNode::Text("<script>alert(\"xss\")</script>".into())]
				.xpipe(RenderHtmlEscaped::default()),
		)
		.to_be("&lt;script&gt;alert(\"xss\")&lt;/script&gt;");
	}
}
