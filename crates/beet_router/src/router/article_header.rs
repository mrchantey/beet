//! The chrome an article wears above its body, rendered from the page's own
//! [`PageMeta`] rather than repeated in every post.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// An article's heading block: its `<h1>`, an `AUTHOR · DATE` byline, and the
/// video the post companions, all read from the page's [`PageMeta`].
///
/// Placed in a LAYOUT (`site/templates/ArticleLayout.bsx`) rather than in each
/// post, so it applies to a whole url subtree and to nothing else, and a post is
/// its prose plus a frontmatter block. Renders nothing for a page with no
/// `created` date, so an undated index page sharing the article layout keeps its
/// own hand-authored heading.
///
/// A layout builds detached from the content it wraps, so the metadata comes
/// from [`RequestContext::content`] — the seam `RouteHead` binds the document
/// `<title>` through, which terminates at the PAGE however deep the layouts
/// nest — not from an in-tree ancestor walk.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so a
/// BSX layout places it with `<ArticleHeader/>`.
#[template(system)]
pub fn ArticleHeader(
	stack: Res<RequestContextStack>,
	metas: Query<&PageMeta>,
) -> impl Bundle {
	let header = metas
		.get(stack.current().content())
		.ok()
		.filter(|meta| meta.created.is_some())
		.map(|meta| {
			let title = meta.title.clone().unwrap_or_default();
			let byline = article_byline(meta);
			let video = meta.video_url.clone().map(|url| {
				rsx! { <YouTubeEmbed url=url title={title.clone()}/> }
			});
			rsx! {
				<h1>{title.clone()}</h1>
				{byline}
				{video}
			}
		});
	rsx! { {header} }
}

/// The `AUTHOR · DATE` byline, or nothing when the page declares neither: a page
/// without one renders no empty paragraph.
///
/// Shared with [`RouteIndex`](crate::prelude::RouteIndex), whose entries wear the
/// same line, so the two cannot drift apart.
pub(crate) fn article_byline(meta: &PageMeta) -> Snippet {
	let text = [
		meta.author.as_deref().map(str::to_uppercase),
		meta.created
			.map(|created| created.format_long_date().to_uppercase()),
	]
	.into_iter()
	.flatten()
	.collect::<Vec<_>>()
	.join(" · ");
	if text.is_empty() {
		return Snippet::from_bundle(());
	}
	rsx! { <p {Classes::new([classes::TEXT_EYEBROW])}>{text}</p> }
}

/// A YouTube video as its player iframe, named by the WATCH url a page's
/// `video_url` carries, since that is the url a human shares.
///
/// The watch url rides along as `alt-src`, so a surface that cannot host an
/// iframe still has the link. A url that names no YouTube video renders nothing
/// rather than an iframe pointing at itself.
///
/// Registered by name (see [`RouterPlugin`](crate::prelude::RouterPlugin)), so a
/// page places it with `<YouTubeEmbed url=".." title=".."/>`.
#[template]
pub fn YouTubeEmbed(
	/// The video's watch url, eg `https://youtu.be/7koepBSRoUI`.
	#[prop(into)]
	url: String,
	/// The iframe's accessible title.
	#[prop(into, default)]
	title: String,
) -> impl Bundle {
	let embed =
		video_id(&url).map(|id| format!("https://www.youtube.com/embed/{id}"));
	rsx! {
		{embed.map(|embed| rsx!{
			<iframe
				src={embed}
				alt-src={url.clone()}
				title={title.clone()}
				frameborder="0"
				allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
				referrerpolicy="strict-origin-when-cross-origin"
				allowfullscreen></iframe>
		})}
	}
}

/// The video id in a YouTube url: the `7koepBSRoUI` in `https://youtu.be/..`,
/// `https://www.youtube.com/watch?v=..` or an already-embed url. `None` for any
/// other url, so a video hosted elsewhere declares no YouTube embed.
fn video_id(url: &str) -> Option<&str> {
	url.split_once("youtu.be/")
		.or_else(|| url.split_once("youtube.com/embed/"))
		.or_else(|| url.split_once("watch?v="))
		.map(|(_, id)| id)?
		.split(['?', '&', '#'])
		.next()
		.filter(|id| !id.is_empty())
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn parses_youtube_urls() {
		video_id("https://youtu.be/7koepBSRoUI")
			.unwrap()
			.xpect_eq("7koepBSRoUI");
		// a share url carries a timestamp, the watch url a playlist
		video_id("https://youtu.be/7koepBSRoUI?t=42")
			.unwrap()
			.xpect_eq("7koepBSRoUI");
		video_id("https://www.youtube.com/watch?v=7koepBSRoUI&list=PL")
			.unwrap()
			.xpect_eq("7koepBSRoUI");
		video_id("https://www.youtube.com/embed/7koepBSRoUI")
			.unwrap()
			.xpect_eq("7koepBSRoUI");
		// a video hosted elsewhere is not a YouTube embed
		video_id("https://example.com/video.mp4").xpect_none();
	}
}
