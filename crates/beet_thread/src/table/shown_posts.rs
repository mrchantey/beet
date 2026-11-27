//! What a person or an agent reads of a thread, see [`ShownPosts`].
use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;

/// A run of a thread without what was edited away or deleted (the model
/// page, [`crate::table`], "Everything and shown"): the same posts as the
/// records, with a deleted post's body and an edit's original emptied.
///
/// Derived in memory and stored nowhere, it is a whole list made in one place
/// ([`ThreadRecords::shown`]) with no way to change a post of it, so a list in
/// hand is known clean and no reader judges a post at a time. A post nobody
/// touched is the records' own, shared rather than copied. A deleted post
/// stays in it, its status `deleted`, so a view can say it was there, and
/// [`AgentPost`] reads it as that and nothing else.
///
/// A view, a composer, an agent's context and its tools, and a
/// [`ThreadRoute`](crate::prelude::ThreadRoute)'s answer read these; the
/// records ([`ThreadRecords::posts`]) and the store hold everything.
#[derive(Debug, Default, Clone, Deref)]
pub struct ShownPosts(Vec<Arc<Post>>);

impl ShownPosts {
	/// The files these posts hold, oldest first, a deleted post's gone with
	/// its body.
	pub fn files(&self) -> impl DoubleEndedIterator<Item = PostFile> {
		self.0.iter().filter_map(|post| post.file())
	}

	/// The post `id`, if shown here.
	pub fn post(&self, id: PostId) -> Option<&Post> {
		self.0
			.binary_search_by_key(&id, |post| post.id())
			.ok()
			.map(|index| &*self.0[index])
	}

	/// `post` as shown: a deleted post and an `edited` log a copy with the
	/// body emptied, any other the records' own.
	fn show(post: &Arc<Post>) -> Arc<Post> {
		match post.is_deleted() || post.kind() == &PostKind::Edited {
			true => Arc::new(post.emptied()),
			false => post.clone(),
		}
	}
}

impl ThreadRecords {
	/// The posts of `span` as shown, a person's and an agent's reading of
	/// the records, so a reader takes only what it needs, ie a view one
	/// changed post at a time.
	pub fn shown(
		&self,
		span: impl core::ops::RangeBounds<PostId>,
	) -> ShownPosts {
		use core::ops::Bound;
		let start = match span.start_bound() {
			Bound::Included(id) => {
				self.posts().partition_point(|post| post.id() < *id)
			}
			Bound::Excluded(id) => {
				self.posts().partition_point(|post| post.id() <= *id)
			}
			Bound::Unbounded => 0,
		};
		let end = match span.end_bound() {
			Bound::Included(id) => {
				self.posts().partition_point(|post| post.id() <= *id)
			}
			Bound::Excluded(id) => {
				self.posts().partition_point(|post| post.id() < *id)
			}
			Bound::Unbounded => self.posts().len(),
		};
		self.posts()[start..end.max(start)]
			.iter()
			.map(ShownPosts::show)
			.collect::<Vec<_>>()
			.xmap(ShownPosts)
	}
}
