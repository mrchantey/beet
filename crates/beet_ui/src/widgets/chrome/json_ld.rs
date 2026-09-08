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
	pub url: Option<String>,
	/// Publication date as ISO 8601. Its presence is what makes the page an
	/// `Article` rather than a `WebPage`.
	pub published: Option<String>,
	/// Last modification as ISO 8601.
	pub modified: Option<String>,
	/// The author's name, rendered as a schema.org `Person`.
	pub author: Option<SmolStr>,
	/// The page's social card image.
	pub image: Option<SmolStr>,
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
		json_ext::object([
			json_ext::member_opt("@context", Some("https://schema.org")),
			json_ext::member_opt("@type", Some(kind)),
			json_ext::member_opt("headline", Some(&self.headline)),
			json_ext::member_opt("url", self.url.as_ref()),
			json_ext::member_opt("datePublished", self.published.as_ref()),
			json_ext::member_opt("dateModified", self.modified.as_ref()),
			self.author.as_ref().map(|author| {
				json_ext::member("author", person("Person", author))
			}),
			json_ext::member_opt("image", self.image.as_ref()),
			Some(json_ext::member(
				"publisher",
				person("Organization", &self.publisher),
			)),
		])
	}
}

/// A schema.org named entity, ie `{"@type":"Person","name":".."}`.
fn person(kind: &str, name: &str) -> String {
	json_ext::object([
		json_ext::member_opt("@type", Some(kind)),
		json_ext::member_opt("name", Some(name)),
	])
}

#[cfg(test)]
mod test {
	use super::*;

	/// A dated page is an `Article` carrying its dates and its author.
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
			r#"{"@context":"https://schema.org","@type":"Article","headline":"ECS Router","url":"https://beet.org/blog/ecs-router","datePublished":"2025-08-09T00:00:00.000Z","dateModified":"2025-09-01T00:00:00.000Z","author":{"@type":"Person","name":"Pete Hayman"},"image":"https://beet.org/card.png","publisher":{"@type":"Organization","name":"Beet"}}"#
				.to_string(),
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
			r#"{"@context":"https://schema.org","@type":"WebPage","headline":"Beet","publisher":{"@type":"Organization","name":"Beet"}}"#
				.to_string(),
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
