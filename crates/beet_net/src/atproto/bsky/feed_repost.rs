//! `app.bsky.feed.repost`, a post shared again by another account.
use beet_core::prelude::*;

/// `app.bsky.feed.repost`: the account holding it shares `subject` with its
/// followers. Keyed by TID, minted on first write; its natural key is the
/// subject's uri, since an account reposts a post at most once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedRepost {
	/// The post reposted.
	pub subject: StrongRef,
	/// When the account reposted it, an RFC 3339 datetime.
	#[serde(rename = "createdAt")]
	pub created_at: SmolStr,
	/// The repost this one was made through, when reposted from a repost.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub via: Option<StrongRef>,
}

impl AtprotoRecord for FeedRepost {
	const COLLECTION: Nsid = Nsid::new_static("app.bsky.feed.repost");
}

impl FeedRepost {
	/// A repost of `subject`, made at `created_at`.
	pub fn new(subject: StrongRef, created_at: Timestamp) -> Self {
		Self {
			subject,
			created_at: created_at.format_iso8601().into(),
			via: None,
		}
	}
}
