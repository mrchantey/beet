//! Hand-rolled XML serialization for the syndication documents.
//!
//! A sitemap and an RSS feed are each a handful of elements over text the site
//! already holds, so they are written directly rather than through a
//! serialization crate: the whole surface is one escape function and one
//! element writer.

use beet_core::prelude::*;

/// `text` with the five XML predefined entities escaped, ie safe as element
/// content or as an attribute value.
///
/// Both `"` and `'` are escaped even though element content needs neither, so
/// one function serves both positions and no caller has to remember which it is
/// in.
pub(crate) fn escape(text: &str) -> String {
	let mut out = String::with_capacity(text.len());
	for char in text.chars() {
		match char {
			'&' => out.push_str("&amp;"),
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			'"' => out.push_str("&quot;"),
			'\'' => out.push_str("&apos;"),
			_ => out.push(char),
		}
	}
	out
}

/// The inverse of [`escape`]: the five predefined entities decoded back to the
/// characters they stand for.
///
/// Read by the search index, which carries a page's rendered prose as text: the
/// document it came from was escaped for markup, and a reader searching for
/// `Tom & Jerry` types the ampersand.
pub(crate) fn unescape(text: &str) -> String {
	text.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&apos;", "'")
		// last, so a literal `&amp;lt;` decodes to `&lt;` rather than to `<`
		.replace("&amp;", "&")
}

/// An `<{name}>{escaped value}</{name}>` element, indented by `indent` tabs and
/// newline-terminated.
pub(crate) fn element(indent: usize, name: &str, value: &str) -> String {
	format!(
		"{}<{name}>{}</{name}>\n",
		"\t".repeat(indent),
		escape(value)
	)
}

/// [`element`] for an optional value: nothing at all when it is `None`, rather
/// than an empty element a reader would have to interpret.
pub(crate) fn element_opt(
	indent: usize,
	name: &str,
	value: Option<impl AsRef<str>>,
) -> String {
	value
		.map(|value| element(indent, name, value.as_ref()))
		.unwrap_or_default()
}

/// The XML declaration every syndication document opens with.
pub(crate) const PROLOG: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn escapes_predefined_entities() {
		escape("Tom & Jerry's <b>\"show\"</b>").xpect_eq(
			"Tom &amp; Jerry&apos;s &lt;b&gt;&quot;show&quot;&lt;/b&gt;"
				.to_string(),
		);
	}

	#[beet_core::test]
	fn round_trips_through_escaping() {
		let text = "Tom & Jerry's <b>\"show\"</b>";
		unescape(&escape(text)).xpect_eq(text.to_string());
		// the ampersand decodes last, so an escaped entity survives one trip
		unescape("&amp;lt;").xpect_eq("&lt;".to_string());
	}

	#[beet_core::test]
	fn writes_elements() {
		element(1, "loc", "https://beet.org/a&b")
			.xpect_eq("\t<loc>https://beet.org/a&amp;b</loc>\n".to_string());
		element_opt(2, "author", None::<&str>).xpect_eq(String::new());
		element_opt(0, "author", Some("Pete"))
			.xpect_eq("<author>Pete</author>\n".to_string());
	}
}
