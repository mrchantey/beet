use crate::prelude::*;
use beet_core::prelude::*;

/// Author a seed [`Post`] in markup as a child of a `<CreateActor>`.
///
/// Expands to a [`SeedPost`]; the author-to-behavior reduction resolves its
/// author (the actor parent) and thread, hoists it into the [`ThreadWindow`],
/// and despawns the entity, leaving only the record.
///
/// With `src` rather than `text` it expands to a [`SeedSource`] reading the
/// file at that path in the store at or above the thread while the thread
/// builds, so a long procedure has one source, ie
/// `<CreatePost src="seeds/system.md"/>`.
///
/// Set `intent` / `status` to author refusals ([`PostIntent::Refusal`]),
/// reasoning ([`PostIntent::Reasoning`]), or in-progress posts
/// ([`PostStatus::InProgress`]); both default to a completed, OK post.
#[template]
pub fn CreatePost(
	/// The post's text.
	text: String,
	/// A file in the store above the thread holding the post's text, read
	/// while the thread builds, in place of `text`.
	src: Option<RelPath>,
	#[prop(default = PostIntent::Message)] intent: PostIntent,
	#[prop(default = PostStatus::Completed)] status: PostStatus,
) -> impl Bundle {
	OnSpawn::new(move |entity| {
		match src {
			#[cfg(feature = "action")]
			Some(src) => {
				entity.insert(SeedSource {
					intent,
					status,
					..SeedSource::blob(src)
				});
			}
			#[cfg(not(feature = "action"))]
			Some(src) => {
				bevybail!(
					"`<CreatePost src=\"{src}\"/>` needs the `action` feature"
				)
			}
			None => {
				entity.insert(Post::spawn_with(text, intent, status));
			}
		}
		Ok(())
	})
}
