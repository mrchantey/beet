//! What changed in a window's records, and since when, see [`Revision`].
use crate::prelude::*;
use beet_core::prelude::*;
use std::collections::BTreeMap;

/// A point in a window's records' history. A reader keeps the revision it
/// last looked at and asks the records what changed since
/// ([`ThreadRecords::changed_since`]), so a view, window sync and the flush
/// each do work proportional to the change, never to the thread.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

/// How a post last changed, each the [`Revision`] it changed at, the default
/// for never.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PostRevision {
	/// Pushed or updated by this window.
	pub local: Revision,
	/// Loaded from the store or received from another window.
	pub received: Revision,
	/// Its response metadata set by this window.
	pub meta: Revision,
}

impl PostRevision {
	/// The newest of its changes.
	fn latest(&self) -> Revision {
		self.local.max(self.received).max(self.meta)
	}
	/// Whether the post itself changed after `since`, however it did.
	pub fn post_after(&self, since: Revision) -> bool {
		self.local > since || self.received > since
	}
	/// Whether this window pushed or updated the post after `since`.
	pub fn local_after(&self, since: Revision) -> bool { self.local > since }
	/// Whether this window set the post's metadata after `since`.
	pub fn meta_after(&self, since: Revision) -> bool { self.meta > since }
}

/// Each post's latest changes, found by the revision they happened at, so
/// what changed since a revision is a range read. One entry a post, so it
/// grows with the records and never with how often they change.
#[derive(Debug, Default, Clone)]
pub(crate) struct RevisionLog {
	/// The revision of the newest change.
	revision: Revision,
	/// How each post last changed.
	posts: HashMap<PostId, PostRevision>,
	/// Each post by its latest revision.
	order: BTreeMap<Revision, PostId>,
}

impl RevisionLog {
	/// The revision of the newest change.
	pub fn revision(&self) -> Revision { self.revision }

	/// How the post `id` last changed.
	fn of(&self, id: PostId) -> PostRevision {
		self.posts.get(&id).copied().unwrap_or_default()
	}

	/// Record a change to `id`, `func` setting which of its revisions it
	/// moves.
	fn bump(
		&mut self,
		id: PostId,
		func: impl FnOnce(&mut PostRevision, Revision),
	) {
		self.revision.0 += 1;
		let revision = self.revision;
		let entry = self.posts.entry(id).or_default();
		self.order.remove(&entry.latest());
		func(entry, revision);
		self.order.insert(revision, id);
	}

	/// This window pushed or updated `id`.
	pub fn local(&mut self, id: PostId) {
		self.bump(id, |post, revision| post.local = revision);
	}
	/// `id` was loaded or received.
	pub fn received(&mut self, id: PostId) {
		self.bump(id, |post, revision| post.received = revision);
	}
	/// This window set the metadata of `id`.
	pub fn meta(&mut self, id: PostId) {
		self.bump(id, |post, revision| post.meta = revision);
	}

	/// Each post that changed after `since`, in the order it last changed.
	pub fn since(
		&self,
		since: Revision,
	) -> impl Iterator<Item = (PostId, PostRevision)> {
		self.order
			.range((
				core::ops::Bound::Excluded(since),
				core::ops::Bound::Unbounded,
			))
			.map(|(_, id)| (*id, self.of(*id)))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A reader asking since a revision gets each post that changed after it
	/// once, as it last changed, and a post changed twice moves to its newest
	/// place.
	#[beet_core::test]
	fn reads_what_changed_since() {
		let mut log = RevisionLog::default();
		let [first, second] = [PostId::now(), PostId::now()];
		log.local(first);
		let seen = log.revision();
		log.received(second);
		log.meta(first);
		let changed = log.since(seen).collect::<Vec<_>>();
		changed
			.iter()
			.map(|(id, _)| *id)
			.collect::<Vec<_>>()
			.xpect_eq(vec![second, first]);
		// the first's own change was seen, its metadata's not
		changed[1].1.local_after(seen).xpect_false();
		changed[1].1.meta_after(seen).xpect_true();
		changed[0].1.post_after(seen).xpect_true();
		changed[0].1.local_after(seen).xpect_false();
		log.since(log.revision()).count().xpect_eq(0);
		log.order.len().xpect_eq(2);
	}
}
