//! HTML's own vocabulary, shared by what reads markup and what writes it: the
//! element kinds, and the conversion both ways between character references
//! and their characters, covering the most common typographic and structural
//! entities from the W3C reference.

use crate::prelude::*;

/// The HTML block-level element names, the single list shared by everything that
/// distinguishes block from inline elements for whitespace and layout decisions
/// (renderers, the markdown tree builder, …).
pub const BLOCK_ELEMENTS: &[&str] = &[
	"address",
	"article",
	"aside",
	"blockquote",
	"details",
	"dialog",
	"dd",
	"div",
	"dl",
	"dt",
	"fieldset",
	"figcaption",
	"figure",
	"footer",
	"form",
	"h1",
	"h2",
	"h3",
	"h4",
	"h5",
	"h6",
	"header",
	"hgroup",
	"hr",
	"li",
	"main",
	"nav",
	"ol",
	"p",
	"pre",
	"search",
	"section",
	"table",
	"ul",
];

/// Whether `name` is an HTML block-level element (case-insensitive), per
/// [`BLOCK_ELEMENTS`].
pub fn is_block_element(name: &str) -> bool {
	BLOCK_ELEMENTS
		.iter()
		.any(|el| el.eq_ignore_ascii_case(name))
}

/// The HTML void element names, those without a closing tag.
pub const VOID_ELEMENTS: &[&str] = &[
	"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
	"param", "source", "track", "wbr",
];

/// The HTML raw text element names, whose children are never escaped.
pub const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

/// `names` as an owned list, for a renderer or parser that stores an
/// overridable one, ie `element_names(VOID_ELEMENTS)`.
pub fn element_names(names: &[&'static str]) -> Vec<SmolStr> {
	names.iter().copied().map(SmolStr::new_static).collect()
}

/// The class of the wrapper every table sits in, the block that scrolls it
/// sideways on a narrow screen.
pub const TABLE_SCROLL_CLASS: &str = "table-scroll";

/// Entity pairs as `(html_entity, character)`.
const HTML_ENTITY_PAIRS: &[(&str, &str)] = &[
	// Structural / required escapes
	("&amp;", "&"),
	("&lt;", "<"),
	("&gt;", ">"),
	("&quot;", "\""),
	("&apos;", "'"),
	// Non-breaking and special spaces
	("&nbsp;", "\u{00A0}"),
	("&ensp;", "\u{2002}"),
	("&emsp;", "\u{2003}"),
	("&thinsp;", "\u{2009}"),
	// Dashes
	("&ndash;", "\u{2013}"),
	("&mdash;", "\u{2014}"),
	// Quotation marks
	("&lsquo;", "\u{2018}"),
	("&rsquo;", "\u{2019}"),
	("&ldquo;", "\u{201C}"),
	("&rdquo;", "\u{201D}"),
	("&sbquo;", "\u{201A}"),
	("&bdquo;", "\u{201E}"),
	("&laquo;", "\u{00AB}"),
	("&raquo;", "\u{00BB}"),
	("&lsaquo;", "\u{2039}"),
	("&rsaquo;", "\u{203A}"),
	// Punctuation and symbols
	("&hellip;", "\u{2026}"),
	("&middot;", "\u{00B7}"),
	("&bull;", "\u{2022}"),
	("&prime;", "\u{2032}"),
	("&Prime;", "\u{2033}"),
	("&sect;", "\u{00A7}"),
	("&para;", "\u{00B6}"),
	("&dagger;", "\u{2020}"),
	("&Dagger;", "\u{2021}"),
	// Currency
	("&cent;", "\u{00A2}"),
	("&pound;", "\u{00A3}"),
	("&yen;", "\u{00A5}"),
	("&euro;", "\u{20AC}"),
	("&curren;", "\u{00A4}"),
	// Math and miscellaneous
	("&times;", "\u{00D7}"),
	("&divide;", "\u{00F7}"),
	("&plusmn;", "\u{00B1}"),
	("&deg;", "\u{00B0}"),
	("&micro;", "\u{00B5}"),
	("&frac12;", "\u{00BD}"),
	("&frac14;", "\u{00BC}"),
	("&frac34;", "\u{00BE}"),
	("&copy;", "\u{00A9}"),
	("&reg;", "\u{00AE}"),
	("&trade;", "\u{2122}"),
];

/// Look up the character replacement for an HTML entity.
fn unescape_entity(entity: &str) -> Option<&'static str> {
	HTML_ENTITY_PAIRS
		.iter()
		.find(|(ent, _)| *ent == entity)
		.map(|(_, ch)| *ch)
}

/// Look up the HTML entity for a character.
fn escape_char(ch: &str) -> Option<&'static str> {
	HTML_ENTITY_PAIRS
		.iter()
		.find(|(_, c)| *c == ch)
		.map(|(ent, _)| *ent)
}

/// Replace HTML entities with their corresponding characters.
pub fn unescape_text(input: &str) -> String {
	let mut result = String::with_capacity(input.len());
	let mut remaining = input;

	while let Some(amp_pos) = remaining.find('&') {
		// Copy everything before the `&`.
		result.push_str(&remaining[..amp_pos]);
		remaining = &remaining[amp_pos..];

		// Look for the closing `;`.
		if let Some(semi_pos) = remaining.find(';') {
			let entity = &remaining[..=semi_pos];
			if let Some(replacement) = unescape_entity(entity) {
				result.push_str(replacement);
				remaining = &remaining[semi_pos + 1..];
			} else {
				// Unrecognized entity, copy the `&` literally and continue.
				result.push('&');
				remaining = &remaining[1..];
			}
		} else {
			// No closing `;` found, copy the rest literally.
			result.push_str(remaining);
			return result;
		}
	}

	result.push_str(remaining);
	result
}

/// Replace special characters with their HTML entity equivalents.
///
/// Only escapes characters present in [`HTML_ENTITY_PAIRS`]. Single
/// characters are matched greedily so multi-byte sequences like
/// `\u{2013}` are handled correctly.
pub fn escape_text(input: &str) -> String {
	let mut result = String::with_capacity(input.len());

	for ch in input.chars() {
		let mut buf = [0u8; 4];
		let ch_str = ch.encode_utf8(&mut buf);
		if let Some(entity) = escape_char(ch_str) {
			result.push_str(entity);
		} else {
			result.push(ch);
		}
	}

	result
}

/// Escape characters that are special inside HTML attribute values.
///
/// Attributes require escaping `&`, `"`, and `'` but do **not** need
/// to escape `<` or `>` (those are only meaningful in text content).
/// This function only escapes the structurally required characters,
/// leaving typographic entities untouched.
pub fn escape_attribute(input: &str) -> String {
	let mut result = String::with_capacity(input.len());
	for ch in input.chars() {
		match ch {
			'&' => result.push_str("&amp;"),
			'"' => result.push_str("&quot;"),
			'\'' => result.push_str("&apos;"),
			other => result.push(other),
		}
	}
	result
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn escape_structural_chars() {
		escape_text("<div class=\"foo\">")
			.xpect_eq("&lt;div class=&quot;foo&quot;&gt;".to_string());
	}

	#[crate::test]
	fn escape_ampersand() {
		escape_text("a & b").xpect_eq("a &amp; b".to_string());
	}

	#[crate::test]
	fn escape_no_special() {
		escape_text("hello world").xpect_eq("hello world".to_string());
	}

	#[crate::test]
	fn escape_typographic_chars() {
		escape_text("hello \u{2014} world")
			.xpect_eq("hello &mdash; world".to_string());
	}

	#[crate::test]
	fn escape_attribute_quotes_and_ampersand() {
		escape_attribute(r#"say "hello" & 'goodbye'"#).xpect_eq(
			"say &quot;hello&quot; &amp; &apos;goodbye&apos;".to_string(),
		);
	}

	#[crate::test]
	fn escape_attribute_preserves_angle_brackets() {
		escape_attribute("<b>bold</b>").xpect_eq("<b>bold</b>".to_string());
	}

	/// The unescape half rides the markdown parser's feature, like the
	/// functions it exercises.
	mod unescape {
		use super::super::*;

		#[crate::test]
		fn structural_entities() {
			unescape_text("&lt;div class=&quot;foo&quot;&gt;")
				.xpect_eq("<div class=\"foo\">".to_string());
		}

		#[crate::test]
		fn typographic_entities() {
			unescape_text("&ldquo;hello&rdquo; &mdash; world")
				.xpect_eq("\u{201C}hello\u{201D} \u{2014} world".to_string());
		}

		#[crate::test]
		fn preserves_unknown_entities() {
			unescape_text("&unknownentity; text")
				.xpect_eq("&unknownentity; text".to_string());
		}

		#[crate::test]
		fn bare_ampersand() {
			unescape_text("AT&T").xpect_eq("AT&T".to_string());
		}

		#[crate::test]
		fn empty() { unescape_text("").xpect_eq("".to_string()); }

		#[crate::test]
		fn roundtrip_escape_unescape() {
			let original = "<p>\"Hello\" & 'goodbye' \u{2014} world</p>";
			let escaped = escape_text(original);
			unescape_text(&escaped).xpect_eq(original.to_string());
		}

		#[crate::test]
		fn roundtrip_unescape_escape() {
			let original = "&lt;p&gt;&quot;Hello&quot; &amp; &apos;goodbye&apos; &mdash; world&lt;/p&gt;";
			let unescaped = unescape_text(original);
			escape_text(&unescaped).xpect_eq(original.to_string());
		}

		#[crate::test]
		fn attribute_structural() {
			unescape_text("&quot;hello&quot; &amp; &apos;world&apos;")
				.xpect_eq("\"hello\" & 'world'".to_string());
		}

		#[crate::test]
		fn roundtrip_attribute_escape_unescape() {
			let original = r#"font-family: "Helvetica" & 'Arial'"#;
			let escaped = escape_attribute(original);
			unescape_text(&escaped).xpect_eq(original.to_string());
		}

		#[crate::test]
		fn double_escaped_ampersand() {
			unescape_text("&amp;amp;").xpect_eq("&amp;".to_string());
		}
	}
}
