//! The Bluesky AppView, read unauthenticated over xrpc.
use crate::prelude::*;
use beet_core::prelude::*;

/// A read-only xrpc client for a Bluesky AppView: hydrated posts, feed pages
/// and threads.
///
/// Unauthenticated: the public instance serves every call here without a
/// session, so a client needs only an http transport. As a component it names
/// the AppView an entity's subtree reads through, resolved by ancestry, the
/// public one where none is declared:
///
/// ```html
/// <Router {AppView{host:"https://appview.example.com"}}>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AppView {
	/// The AppView's origin, ie [`AppView::PUBLIC`].
	pub host: SmolStr,
}

impl Default for AppView {
	fn default() -> Self { Self::new(Self::PUBLIC) }
}

impl AppView {
	/// The public unauthenticated Bluesky AppView, the one the app itself
	/// asks.
	pub const PUBLIC: &'static str = "https://public.api.bsky.app";
	/// The most uris one `getPosts` call takes.
	pub const MAX_GET_POSTS_URIS: usize = 25;
	/// Bluesky's Discover feed, a stable published generator for demos and
	/// live tests of [`AppView::get_feed`].
	pub const DISCOVER_FEED: &'static str = "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot";

	/// A client of the AppView at `host`, ie a self-hosted one.
	pub fn new(host: impl Into<SmolStr>) -> Self {
		Self {
			host: SmolStr::new(host.into().trim_end_matches('/')),
		}
	}

	/// Hydrate post uris into [`PostView`]s, in the order requested.
	///
	/// Uris are chunked to [`AppView::MAX_GET_POSTS_URIS`] per call. The
	/// AppView omits deleted and otherwise missing posts, so the result may be
	/// shorter than the request.
	pub async fn get_posts(&self, uris: &[AtUri]) -> Result<Vec<PostView>> {
		let mut posts = Vec::with_capacity(uris.len());
		for chunk in uris.chunks(Self::MAX_GET_POSTS_URIS) {
			posts.extend(
				self.get_posts_request(chunk)
					.send()
					.await?
					.into_result()
					.await?
					.json::<GetPostsResponse>()
					.await?
					.posts,
			);
		}
		Self::order_posts(uris, posts).xok()
	}

	/// A page of a *published* feed, hydrated by the AppView in one call.
	///
	/// The AppView proxies the generator itself, so this only resolves feeds
	/// with a declaration record, erroring `UnknownFeed` otherwise.
	pub async fn get_feed(
		&self,
		feed: &str,
		limit: usize,
		cursor: Option<&str>,
	) -> Result<GetFeedResponse> {
		let request = self
			.xrpc("app.bsky.feed.getFeed")
			.with_param("feed", feed)
			.with_param("limit", &limit.to_string());
		match cursor {
			Some(cursor) => request.with_param("cursor", cursor),
			None => request,
		}
		.send()
		.await?
		.into_result()
		.await?
		.json::<GetFeedResponse>()
		.await
	}

	/// The thread around the post at `uri`: its replies `depth` levels down
	/// and its parents `parent_height` levels up.
	///
	/// # Errors
	/// Errors when the AppView does not answer, or answers that it has no
	/// post at `uri`.
	pub async fn get_post_thread(
		&self,
		uri: &AtUri,
		depth: u16,
		parent_height: u16,
	) -> Result<PostThread> {
		self.xrpc("app.bsky.feed.getPostThread")
			.with_param("uri", &uri.to_string())
			.with_param("depth", &depth.to_string())
			.with_param("parentHeight", &parent_height.to_string())
			.send()
			.await?
			.into_result()
			.await
			.map_err(|err| {
				bevyhow!(
					"reading the thread of {uri} from {}: {err}",
					self.host
				)
			})?
			.json::<GetPostThreadResponse>()
			.await?
			.thread
			.xok()
	}

	/// A `GET` of the xrpc method `nsid` on this AppView.
	fn xrpc(&self, nsid: &str) -> Request {
		Request::get(format!("{}/xrpc/{nsid}", self.host))
	}

	/// The `getPosts` request for one chunk, repeating the `uris` param.
	fn get_posts_request(&self, chunk: &[AtUri]) -> Request {
		chunk
			.iter()
			.fold(self.xrpc("app.bsky.feed.getPosts"), |request, uri| {
				request.with_param("uris", &uri.to_string())
			})
	}

	/// Restore the requested order, dropping uris the AppView did not return.
	fn order_posts(uris: &[AtUri], posts: Vec<PostView>) -> Vec<PostView> {
		let mut by_uri = posts
			.into_iter()
			.map(|post| (post.uri.clone(), post))
			.collect::<HashMap<_, _>>();
		uris.iter().filter_map(|uri| by_uri.remove(uri)).collect()
	}
}

/// The `app.bsky.feed.getFeed` output: a hydrated page and its continuation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetFeedResponse {
	/// The cursor for the next page, absent at the end of the feed.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cursor: Option<SmolStr>,
	/// The page of posts, newest first.
	pub feed: Vec<FeedViewPost>,
}

/// The `app.bsky.feed.getPosts` output.
#[derive(Debug, Deserialize)]
struct GetPostsResponse {
	posts: Vec<PostView>,
}

/// The `app.bsky.feed.getPostThread` output, the parts read.
#[derive(Debug, Deserialize)]
struct GetPostThreadResponse {
	thread: PostThread,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A post view as the AppView serves it: every required key plus the
	/// optional display name and counters.
	const POST_VIEW: &str = r#"{
		"uri": "at://did:plc:hnv7bd4gtxrf7iigjo22qukp/app.bsky.feed.post/3l3qo2vutsw2b",
		"cid": "bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza",
		"author": {
			"did": "did:plc:hnv7bd4gtxrf7iigjo22qukp",
			"handle": "alice.bsky.social",
			"displayName": "Alice",
			"avatar": "https://cdn.bsky.app/img/avatar/alice.jpg"
		},
		"record": {
			"$type": "app.bsky.feed.post",
			"text": "have you seen alf lately?",
			"createdAt": "2024-09-09T19:46:02.102Z"
		},
		"replyCount": 2,
		"likeCount": 4,
		"indexedAt": "2024-09-09T19:46:03.000Z"
	}"#;

	fn uri(rkey: &str) -> AtUri {
		AtUri::parse(&format!(
			"at://did:plc:hnv7bd4gtxrf7iigjo22qukp/app.bsky.feed.post/{rkey}"
		))
		.unwrap()
	}

	fn post_view(rkey: &str) -> PostView {
		PostView {
			uri: uri(rkey),
			..serde_json::from_str(POST_VIEW).unwrap()
		}
	}

	#[beet_core::test]
	fn reads_a_post_view() {
		let post = serde_json::from_str::<PostView>(POST_VIEW).unwrap();
		post.author.label().xpect_eq("Alice");
		post.record.text.xpect_eq("have you seen alf lately?");
		post.like_count.xpect_eq(4);
		// absent counters default rather than failing the whole page
		post.quote_count.xpect_eq(0);
	}

	/// A thread's unions: a missing reply and an unknown view read beside a
	/// post, and only the post is a reply shown.
	#[beet_core::test]
	fn reads_a_thread() {
		let thread = serde_json::from_str::<PostThread>(&format!(
			r#"{{
				"$type": "app.bsky.feed.defs#threadViewPost",
				"post": {POST_VIEW},
				"replies": [
					{{ "$type": "app.bsky.feed.defs#notFoundPost", "uri": "{}", "notFound": true }},
					{{ "$type": "app.bsky.feed.defs#someFutureView" }},
					{{ "$type": "app.bsky.feed.defs#threadViewPost", "post": {POST_VIEW} }}
				]
			}}"#,
			uri("3l3gone")
		))
		.unwrap();
		let root = thread.post().unwrap();
		root.replies.len().xpect_eq(3);
		root.replies_chronological().len().xpect_eq(1);
	}

	/// The uris array param repeats, chunked to the method's limit.
	#[beet_core::test]
	fn chunks_get_posts() {
		let uris = (0..AppView::MAX_GET_POSTS_URIS + 2)
			.map(|index| uri(&format!("3l3{index:010}")))
			.collect::<Vec<_>>();
		let chunks =
			uris.chunks(AppView::MAX_GET_POSTS_URIS).collect::<Vec<_>>();
		chunks.len().xpect_eq(2);
		AppView::default()
			.get_posts_request(chunks[1])
			.url()
			.to_string()
			.xpect_starts_with(
				"https://public.api.bsky.app/xrpc/app.bsky.feed.getPosts?uris=",
			)
			.xpect_contains("3l30000000025")
			.xpect_contains("3l30000000026")
			.xnot()
			.xpect_contains("3l30000000024");
	}

	#[beet_core::test]
	fn orders_posts_and_drops_missing() {
		let uris = [uri("3l3a"), uri("3l3b"), uri("3l3c")];
		// the AppView answered out of order, and dropped the deleted `b`
		AppView::order_posts(&uris, vec![post_view("3l3c"), post_view("3l3a")])
			.into_iter()
			.map(|post| post.uri)
			.collect::<Vec<_>>()
			.xpect_eq(vec![uri("3l3a"), uri("3l3c")]);
	}
}
