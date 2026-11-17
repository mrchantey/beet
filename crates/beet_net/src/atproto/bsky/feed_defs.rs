//! `app.bsky.feed.defs`, the post views an AppView hydrates: a post with its
//! author and counts, a feed page's entry, and a thread.
use crate::prelude::*;
use beet_core::prelude::*;

/// `app.bsky.feed.defs#postView`: a hydrated post, the record plus its author
/// and engagement counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostView {
	/// The post's address.
	pub uri: AtUri,
	/// The record's cid.
	pub cid: Cid,
	/// The posting account.
	pub author: ProfileViewBasic,
	/// The `app.bsky.feed.post` record.
	pub record: FeedPost,
	/// The number of replies, absent unless non-zero.
	#[serde(default, rename = "replyCount")]
	pub reply_count: u64,
	/// The number of reposts, absent unless non-zero.
	#[serde(default, rename = "repostCount")]
	pub repost_count: u64,
	/// The number of likes, absent unless non-zero.
	#[serde(default, rename = "likeCount")]
	pub like_count: u64,
	/// The number of quote posts, absent unless non-zero.
	#[serde(default, rename = "quoteCount")]
	pub quote_count: u64,
	/// When the AppView indexed the post, an RFC 3339 datetime.
	#[serde(default, rename = "indexedAt")]
	pub indexed_at: SmolStr,
}

/// `app.bsky.feed.defs#feedViewPost`: one entry of a hydrated feed page. The
/// reply and repost context served beside `post` is not read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedViewPost {
	/// The post itself.
	pub post: PostView,
}

/// The union a thread's root, parent and replies are each one of:
/// `app.bsky.feed.defs#threadViewPost`, `#notFoundPost` or `#blockedPost`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum PostThread {
	/// A post, with what it replies to and its replies.
	#[serde(rename = "app.bsky.feed.defs#threadViewPost")]
	Post(Box<ThreadViewPost>),
	/// A post the AppView no longer has, ie deleted.
	#[serde(rename = "app.bsky.feed.defs#notFoundPost")]
	NotFound {
		/// The missing post's address.
		uri: AtUri,
	},
	/// A post hidden by a block between its author and the viewer.
	#[serde(rename = "app.bsky.feed.defs#blockedPost")]
	Blocked {
		/// The blocked post's address.
		uri: AtUri,
	},
	/// A view this build does not know, kept so the rest of the thread reads.
	#[serde(other)]
	Other,
}

impl PostThread {
	/// The post this view shows, `None` for a missing, blocked or unknown one.
	pub fn post(&self) -> Option<&ThreadViewPost> {
		match self {
			Self::Post(post) => Some(post),
			_ => None,
		}
	}
}

/// `app.bsky.feed.defs#threadViewPost`: a post in a thread, its parent as far
/// up as the request's `parentHeight` and its replies as far down as its
/// `depth`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadViewPost {
	/// The post itself.
	pub post: PostView,
	/// The post this one replies to.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent: Option<PostThread>,
	/// The replies to this post, in the AppView's order.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub replies: Vec<PostThread>,
}

impl ThreadViewPost {
	/// The replies the AppView shows, oldest first by when each was written,
	/// skipping the missing and the blocked.
	pub fn replies_chronological(&self) -> Vec<&ThreadViewPost> {
		let mut replies = self
			.replies
			.iter()
			.filter_map(PostThread::post)
			.collect::<Vec<_>>();
		replies.sort_by_key(|reply| {
			Timestamp::parse_rfc3339(&reply.post.record.created_at)
		});
		replies
	}
}
