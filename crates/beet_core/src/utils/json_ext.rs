//! Writing small, fixed-shape JSON documents by hand.
//!
//! Not a replacement for serde: it is what a *generator* of a known document
//! needs — a sitemap's sibling search index, a `<script type="application/ld+json">`
//! block — where a `Serialize` impl would only buy a `json` feature gate on
//! output that must be produced in every build. Reading json stays serde's job.

use crate::prelude::*;

/// `value` as a JSON string literal, quotes included.
///
/// `<` is escaped as `\u003c` beyond the JSON minimum, since these documents are
/// routinely embedded in a `<script>` element, where a `</script>` in any value
/// would close the block early and spill the rest into the page as markup.
pub fn string(value: &str) -> String {
	let mut out = String::with_capacity(value.len() + 2);
	out.push('"');
	for char in value.chars() {
		match char {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'<' => out.push_str("\\u003c"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			char if (char as u32) < 0x20 => {
				out.push_str(&format!("\\u{:04x}", char as u32))
			}
			char => out.push(char),
		}
	}
	out.push('"');
	out
}

/// A `"key":value` member, where `value` is already serialized.
pub fn member(key: &str, value: impl AsRef<str>) -> String {
	format!("{}:{}", string(key), value.as_ref())
}

/// A `"key":"string"` member, or nothing when the value is absent.
///
/// An unauthored key is left OUT rather than written `null`: a reader branches
/// on presence either way, and the document stays small.
pub fn member_opt(key: &str, value: Option<impl AsRef<str>>) -> Option<String> {
	value.map(|value| member(key, string(value.as_ref())))
}

/// The present `members` as a `{..}` object.
pub fn object(members: impl IntoIterator<Item = Option<String>>) -> String {
	format!(
		"{{{}}}",
		members.into_iter().flatten().collect::<Vec<_>>().join(",")
	)
}

/// Already-serialized `items` as a `[..]` array.
pub fn array(items: impl IntoIterator<Item = String>) -> String {
	format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn escapes_strings() {
		json_ext::string("plain").xpect_eq("\"plain\"".to_string());
		// a value may not close the `<script>` element it is embedded in
		json_ext::string("</script>\n\t\"x\"")
			.xpect_eq("\"\\u003c/script>\\n\\t\\\"x\\\"\"".to_string());
		json_ext::string("\u{1}").xpect_eq("\"\\u0001\"".to_string());
	}

	/// An absent member is omitted rather than written `null`.
	#[crate::test]
	fn writes_documents() {
		json_ext::array([
			json_ext::object([
				json_ext::member_opt("url", Some("/a")),
				json_ext::member_opt("title", None::<&str>),
				Some(json_ext::member("count", "2")),
			]),
			json_ext::object([json_ext::member_opt("url", Some("/b"))]),
		])
		.xpect_eq(r#"[{"url":"/a","count":2},{"url":"/b"}]"#.to_string());
	}
}
