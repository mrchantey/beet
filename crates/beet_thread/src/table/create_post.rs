use crate::prelude::*;
use beet_core::prelude::*;

/// Author a seed [`Post`] in markup as a child of an `<Actor>`.
///
/// Expands to a [`SeedPost`]; the author-to-behavior reduction resolves its
/// author (the actor parent) and thread, hoists it into the [`ThreadWindow`],
/// and despawns the entity, leaving only the record.
///
/// Set `intent` / `status` to author refusals ([`PostIntent::REFUSAL`]),
/// reasoning ([`PostIntent::REASONING_CONTENT`]), or in-progress posts
/// ([`PostStatus::InProgress`]); both default to a completed, OK post.
#[template]
pub fn CreatePost(
	#[prop(into)] text: String,
	#[prop(default = PostIntent::OK)] intent: PostIntent,
	#[prop(default = PostStatus::Completed)] status: PostStatus,
) -> impl Bundle {
	Post::spawn_with(text, intent, status)
}
