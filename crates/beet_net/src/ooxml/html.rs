//! The HTML the transcoders write, escaped as the HTML parser reads it.

/// Text escaped for an HTML element.
pub(super) fn text(text: &str) -> String {
	quick_xml::escape::partial_escape(text).into_owned()
}

/// A value escaped for an HTML attribute.
pub(super) fn attribute(value: &str) -> String {
	quick_xml::escape::escape(value).into_owned()
}

/// A table of plain text cells headed by its first row, a cell's lines
/// broken with `<br>`.
pub(super) fn table(rows: &[Vec<String>]) -> String {
	let mut out = "<table>\n".to_string();
	for (index, row) in rows.iter().enumerate() {
		let tag = match index {
			0 => "th",
			_ => "td",
		};
		out.push_str("<tr>");
		for cell in row {
			let lines = cell.split('\n').map(text).collect::<Vec<_>>();
			out.push_str(&format!("<{tag}>{}</{tag}>", lines.join("<br>")));
		}
		out.push_str("</tr>\n");
	}
	out.push_str("</table>\n");
	out
}
