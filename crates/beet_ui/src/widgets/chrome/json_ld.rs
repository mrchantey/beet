//! The schema.org description of a page, as the `<script type="application/ld+json">`
//! block the document [`Head`](crate::prelude::Head) emits.

use beet_core::prelude::*;

/// A page described as schema.org structured data: the machine-readable twin of
/// the `<head>`'s own meta tags.
///
/// The tags say what a link preview shows; this says what the page IS, which is
/// what earns a search result its byline, its date and its author rather than a
/// bare title and snippet.
///
/// Hand-serialized rather than derived: the shape is a handful of fixed keys, so
/// a serde dependency here would only buy a feature gate on a block that should
/// render in every build.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct JsonLd {
	/// The page's headline, ie its own title else the site's.
	pub headline: String,
	/// The absolute page url, absent when the site names no origin.
	pub url: Option<Url>,
	/// Publication date as ISO 8601. Its presence is what makes the page an
	/// `Article` rather than a `WebPage`.
	pub published: Option<String>,
	/// Last modification as ISO 8601.
	pub modified: Option<String>,
	/// The author's name, rendered as a schema.org `Person`.
	pub author: Option<SmolStr>,
	/// The page's social card image.
	pub image: Option<Url>,
	/// The site name, ie the publisher of every page on it.
	pub publisher: SmolStr,
}

impl JsonLd {
	/// This description as a schema.org JSON document.
	///
	/// A dated page is an `Article`, anything else a `WebPage`: the same fact
	/// the `og:type` tag reports, read from the same field, so the two cannot
	/// disagree.
	pub fn to_json(&self) -> String {
		let kind = match self.published.is_some() {
			true => "Article",
			false => "WebPage",
		};
		let mut map = Map::default();
		map.insert("@context", Value::str("https://schema.org"));
		map.insert("@type", Value::str(kind));
		map.insert("headline", Value::str(self.headline.as_str()));
		map.insert("publisher", named("Organization", &self.publisher));
		for (key, value) in [
			("url", self.url.as_ref().map(|url| url.to_string())),
			("datePublished", self.published.clone()),
			("dateModified", self.modified.clone()),
			("image", self.image.as_ref().map(|url| url.to_string())),
		] {
			if let Some(value) = value {
				map.insert(key, Value::str(value));
			}
		}
		if let Some(author) = &self.author {
			map.insert("author", named("Person", author));
		}
		// this document is embedded in a `<script>`, where a `</script>` in any
		// value would close the block early and spill the rest into the page as
		// markup. Inside json text a `<` only ever appears within a string
		// literal, so escaping every one is both safe and sufficient.
		Value::Map(map).to_json_string().replace('<', "\\u003c")
	}
}

/// A schema.org named entity, ie `{"@type":"Person","name":".."}`.
fn named(kind: &str, name: &str) -> Value {
	let mut map = Map::default();
	map.insert("@type", Value::str(kind));
	map.insert("name", Value::str(name));
	Value::Map(map)
}

#[cfg(test)]
mod test {
	use super::*;

	/// A dated page is an `Article` carrying its dates and its author. Keys are
	/// written sorted, the one order that is the same every render.
	#[beet_core::test]
	fn describes_an_article() {
		JsonLd {
			headline: "ECS Router".into(),
			url: Some("https://beet.org/blog/ecs-router".into()),
			published: Some("2025-08-09T00:00:00.000Z".into()),
			modified: Some("2025-09-01T00:00:00.000Z".into()),
			author: Some("Pete Hayman".into()),
			image: Some("https://beet.org/card.png".into()),
			publisher: "Beet".into(),
		}
		.to_json()
		.xpect_eq(
			r#"{"@context":"https://schema.org","@type":"Article","author":{"@type":"Person","name":"Pete Hayman"},"dateModified":"2025-09-01T00:00:00.000Z","datePublished":"2025-08-09T00:00:00.000Z","headline":"ECS Router","image":"https://beet.org/card.png","publisher":{"@type":"Organization","name":"Beet"},"url":"https://beet.org/blog/ecs-router"}"#,
		);
	}

	/// An undated page is a `WebPage`, with nothing invented for the fields it
	/// declares nothing for.
	#[beet_core::test]
	fn describes_a_page() {
		JsonLd {
			headline: "Beet".into(),
			publisher: "Beet".into(),
			..default()
		}
		.to_json()
		.xpect_eq(
			r#"{"@context":"https://schema.org","@type":"WebPage","headline":"Beet","publisher":{"@type":"Organization","name":"Beet"}}"#,
		);
	}

	/// A value may not close the `<script>` element it lives in, whatever an
	/// author put in the frontmatter.
	#[beet_core::test]
	fn escapes_script_terminators() {
		JsonLd {
			headline: "</script><img onerror=alert(1)>".into(),
			publisher: "Beet".into(),
			..default()
		}
		.to_json()
		.xpect_contains(
			r#""headline":"\u003c/script>\u003cimg onerror=alert(1)>""#,
		)
		.xnot()
		.xpect_contains("</script>");
	}
}
