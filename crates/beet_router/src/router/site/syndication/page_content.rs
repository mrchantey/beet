//! Rendering a page in-process, and reducing it to the two forms syndication
//! carries.

use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A rendered page's article: its own markup, without the document chrome that
/// would otherwise repeat under every feed entry and swamp every search result.
///
/// The feed carries the [`html`](Self::html) and the search index carries the
/// [`text`](Self::text) reduced from it, both off this ONE extraction, so the
/// two consumers can never disagree about where a page's content begins.
pub(crate) struct PageContent {
	/// The inner HTML of the page's article element, ie its body without the
	/// head, nav or footer.
	pub html: String,
}

impl PageContent {
	/// How much plain text one page contributes to a search index. Client-side
	/// search matches on the opening prose in practice, and the index is
	/// fetched whole by every visitor, so the tail is cost without benefit.
	const TEXT_LIMIT: usize = 8 * 1024;

	/// Render the page at `path` by issuing a real in-process request into
	/// `router`, exactly as static export renders a page, and reduce the result.
	///
	/// `None` when the page does not answer `200`, with a warning naming it: one
	/// broken page drops its own entry rather than failing the whole document.
	/// Recursion is not a hazard — a page never requests the feed.
	pub async fn render(
		world: &AsyncWorld,
		router: Entity,
		path: &SmolPath,
	) -> Option<Self> {
		let entity = world
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				find_router,
				router,
			)
			.await
			.map_err(BevyError::from)
			.flatten()
			.map(|router| world.entity(router))
			.inspect_err(|err| warn!("syndication: no router to render: {err}"))
			.ok()?;
		let request = Request::get(path.with_leading_slash())
			.with_accept(MediaType::Html);
		let html = entity
			.exchange(request)
			.await
			.into_result()
			.await
			.inspect_err(|err| {
				warn!(
					"syndication: skipping '{path}', it failed to render: {err}"
				)
			})
			.ok()?
			.text()
			.await
			.inspect_err(|err| {
				warn!(
					"syndication: skipping '{path}', its body is not text: {err}"
				)
			})
			.ok()?;
		Some(Self::extract(&html))
	}

	/// The article portion of a rendered `document`.
	pub fn extract(document: &str) -> Self {
		Self {
			html: article_html(document).to_string(),
		}
	}

	/// The article as plain text: its prose with markup, scripts and styles
	/// dropped, entities decoded and whitespace collapsed to single spaces,
	/// capped at [`TEXT_LIMIT`](Self::TEXT_LIMIT).
	///
	/// Parsed rather than sliced, the inverse of the html: what a search index
	/// wants is the prose, so the structure has to be understood to be
	/// discarded. An article that will not parse contributes no text rather
	/// than its raw markup.
	pub fn text(&self) -> String {
		let Ok(nodes) =
			BsxNode::parse_document(&self.html, &BsxParseConfig::html())
				.inspect_err(|err| {
					warn!("syndication: unparseable page body: {err}")
				})
		else {
			return String::new();
		};
		let mut runs = Vec::new();
		push_text(&nodes, &mut runs);
		let text = unescape(&runs.join(" "));
		let mut text: String =
			text.split_whitespace().collect::<Vec<_>>().join(" ");
		if text.len() > Self::TEXT_LIMIT {
			// a char boundary at or below the cap, so a multi-byte glyph is
			// never split down the middle
			let end = (0..=Self::TEXT_LIMIT)
				.rev()
				.find(|index| text.is_char_boundary(*index))
				.unwrap_or_default();
			text.truncate(end);
		}
		text
	}
}

/// The inner HTML of a document's article: its `<main>`, else its `<article>`,
/// else the whole document for a page that wraps its content in neither.
///
/// A slice rather than a parse-and-reserialize: the markup a feed reader
/// renders should be the markup the site rendered, character for character, and
/// there is no round trip that guarantees that.
fn article_html(document: &str) -> &str {
	["main", "article"]
		.into_iter()
		.find_map(|tag| inner_html(document, tag))
		.unwrap_or(document)
}

/// The inner HTML of the first `<{tag}>` element in `document`.
///
/// Neither `<main>` nor `<article>` nests in practice, so the first opening tag
/// pairs with the first closing one.
fn inner_html<'a>(document: &'a str, tag: &str) -> Option<&'a str> {
	let open =
		document
			.match_indices(&format!("<{tag}"))
			.find(|(index, _)| {
				// `<mainly>` is not `<main>`: the tag ends at the name
				document[index + tag.len() + 1..]
					.chars()
					.next()
					.is_some_and(|char| {
						char == '>' || char == '/' || char.is_whitespace()
					})
			})?;
	let start = open.0 + document[open.0..].find('>')? + 1;
	let end = start + document[start..].find(&format!("</{tag}"))?;
	Some(&document[start..end])
}

/// Collect every text run in `nodes`, skipping the elements whose content is
/// code rather than prose.
fn push_text(nodes: &[BsxNode], runs: &mut Vec<String>) {
	for node in nodes {
		match node {
			BsxNode::Text(text) if !text.trim().is_empty() => {
				runs.push(text.clone())
			}
			BsxNode::Element(element)
				if !matches!(element.tag.as_str(), "script" | "style") =>
			{
				push_text(&element.children, runs)
			}
			_ => {}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn extracts_the_article() {
		let content = PageContent::extract(
			"<html><head><title>x</title></head><body><nav>Home</nav>\
			 <main><h1>Hello &amp; welcome</h1><p>Some <em>prose</em>.</p>\
			 <script>let ignored = 1;</script></main><footer>Bye</footer>\
			 </body></html>",
		);
		// the chrome is gone and the article's own markup is verbatim
		content.html.xpect_eq(
			"<h1>Hello &amp; welcome</h1><p>Some <em>prose</em>.</p>\
			 <script>let ignored = 1;</script>"
				.to_string(),
		);
		// ..while the text is prose alone, entities decoded, script dropped
		content
			.text()
			.xpect_eq("Hello & welcome Some prose .".to_string());
	}

	/// `<article>` is the fallback for a page that names no `<main>`, and a
	/// page with neither contributes its whole body.
	#[beet_core::test]
	fn falls_back_through_the_wrappers() {
		PageContent::extract("<body><article><p>Post</p></article></body>")
			.html
			.xpect_eq("<p>Post</p>".to_string());
		PageContent::extract("<p>Bare</p>")
			.text()
			.xpect_eq("Bare".to_string());
		// a tag that merely starts with the name is not the wrapper
		PageContent::extract("<mainframe><p>Not main</p></mainframe>")
			.text()
			.xpect_eq("Not main".to_string());
	}
}
