//! Shared HTML entity escape and unescape utilities.
//!
//! Provides bidirectional conversion between HTML entities and their
//! corresponding characters, covering the most common typographic
//! and structural entities from the W3C reference.

use beet_core::prelude::*;
use std::borrow::Cow;
use std::sync::LazyLock;


/// The default set of HTML block-level element names.
///
/// Shared by renderers that need to distinguish block vs inline elements
/// for whitespace and layout decisions.
pub fn default_block_elements() -> Vec<Cow<'static, str>> {
	vec![
		"address".into(),
		"article".into(),
		"aside".into(),
		"blockquote".into(),
		"details".into(),
		"dialog".into(),
		"dd".into(),
		"div".into(),
		"dl".into(),
		"dt".into(),
		"fieldset".into(),
		"figcaption".into(),
		"figure".into(),
		"footer".into(),
		"form".into(),
		"h1".into(),
		"h2".into(),
		"h3".into(),
		"h4".into(),
		"h5".into(),
		"h6".into(),
		"header".into(),
		"hgroup".into(),
		"hr".into(),
		"li".into(),
		"main".into(),
		"nav".into(),
		"ol".into(),
		"p".into(),
		"pre".into(),
		"search".into(),
		"section".into(),
		"table".into(),
		"ul".into(),
	]
}


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

/// Map from HTML entity to character, ie `"&amp;"` → `"&"`.
static UNESCAPE_MAP: LazyLock<HashMap<&'static str, &'static str>> =
	LazyLock::new(|| {
		HTML_ENTITY_PAIRS
			.iter()
			.map(|(entity, ch)| (*entity, *ch))
			.collect()
	});

/// Map from character to HTML entity, ie `"&"` → `"&amp;"`.
static ESCAPE_MAP: LazyLock<HashMap<&'static str, &'static str>> =
	LazyLock::new(|| {
		HTML_ENTITY_PAIRS
			.iter()
			.map(|(entity, ch)| (*ch, *entity))
			.collect()
	});

/// Replace HTML entities with their corresponding characters.
///
/// ```rust
/// # use beet_node::prelude::*;
/// # use beet_core::prelude::*;
/// unescape_html_text("&lt;div&gt;").xpect_eq("<div>".to_string());
/// unescape_html_text("&amp;amp;").xpect_eq("&amp;".to_string());
/// unescape_html_text("no entities").xpect_eq("no entities".to_string());
/// ```
pub fn unescape_html_text(input: &str) -> String {
	let map = &*UNESCAPE_MAP;
	let mut result = String::with_capacity(input.len());
	let mut remaining = input;

	while let Some(amp_pos) = remaining.find('&') {
		// Copy everything before the `&`.
		result.push_str(&remaining[..amp_pos]);
		remaining = &remaining[amp_pos..];

		// Look for the closing `;`.
		if let Some(semi_pos) = remaining.find(';') {
			let entity = &remaining[..=semi_pos];
			if let Some(replacement) = map.get(entity) {
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
///
/// ```rust
/// # use beet_node::prelude::*;
/// # use beet_core::prelude::*;
/// escape_html_text("<div>").xpect_eq("&lt;div&gt;".to_string());
/// escape_html_text("a & b").xpect_eq("a &amp; b".to_string());
/// escape_html_text("no special").xpect_eq("no special".to_string());
/// ```
pub fn escape_html_text(input: &str) -> String {
	let map = &*ESCAPE_MAP;
	let mut result = String::with_capacity(input.len());

	for ch in input.chars() {
		let mut buf = [0u8; 4];
		let ch_str = ch.encode_utf8(&mut buf);
		if let Some(entity) = map.get(ch_str) {
			result.push_str(entity);
		} else {
			result.push(ch);
		}
	}

	result
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn unescape_structural_entities() {
		unescape_html_text("&lt;div class=&quot;foo&quot;&gt;")
			.xpect_eq("<div class=\"foo\">".to_string());
	}

	#[test]
	fn unescape_typographic_entities() {
		unescape_html_text("&ldquo;hello&rdquo; &mdash; world")
			.xpect_eq("\u{201C}hello\u{201D} \u{2014} world".to_string());
	}

	#[test]
	fn unescape_preserves_unknown_entities() {
		unescape_html_text("&unknownentity; text")
			.xpect_eq("&unknownentity; text".to_string());
	}

	#[test]
	fn unescape_bare_ampersand() {
		unescape_html_text("AT&T").xpect_eq("AT&T".to_string());
	}

	#[test]
	fn unescape_empty() { unescape_html_text("").xpect_eq("".to_string()); }

	#[test]
	fn escape_structural_chars() {
		escape_html_text("<div class=\"foo\">")
			.xpect_eq("&lt;div class=&quot;foo&quot;&gt;".to_string());
	}

	#[test]
	fn escape_ampersand() {
		escape_html_text("a & b").xpect_eq("a &amp; b".to_string());
	}

	#[test]
	fn escape_no_special() {
		escape_html_text("hello world").xpect_eq("hello world".to_string());
	}

	#[test]
	fn escape_typographic_chars() {
		escape_html_text("hello \u{2014} world")
			.xpect_eq("hello &mdash; world".to_string());
	}

	#[test]
	fn roundtrip_escape_unescape() {
		let original = "<p>\"Hello\" & 'goodbye' \u{2014} world</p>";
		let escaped = escape_html_text(original);
		unescape_html_text(&escaped).xpect_eq(original.to_string());
	}

	#[test]
	fn roundtrip_unescape_escape() {
		let original = "&lt;p&gt;&quot;Hello&quot; &amp; &apos;goodbye&apos; &mdash; world&lt;/p&gt;";
		let unescaped = unescape_html_text(original);
		escape_html_text(&unescaped).xpect_eq(original.to_string());
	}
}
