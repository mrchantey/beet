use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

/// A window's records: the actors, posts and response metadata of the
/// stored thread it is open onto, held in memory.
///
/// A copy of the [`ThreadStore`], never a truth of its own: a contiguous run
/// of the thread ending at its newest post, and the seed before it where the
/// window holds one apart from the run ([`Self::has_older`]). The window's
/// behavior streams into it, each model call derives its input from it
/// ([`Self::model_posts`]), and its readers, a view, window sync and the
/// flush, each ask what changed since they last looked
/// ([`Self::changed_since`]), so each does work proportional to the change.
///
/// The records hold everything, a deleted post's body and an edit's original
/// included ([`Self::posts`]). What a person or an agent reads is shown
/// ([`Self::shown`]), the same posts with those emptied, so a reader handed
/// a [`ShownPosts`] is known clean.
///
/// Author resolution is in memory (`records.actor(post.author())`), with no
/// entity walk; ordering is by [`PostId`], a time sortable [`Tid`]. Loading
/// another runtime's posts lifts this process's minter past the newest of
/// them, so a post minted after is after every post the window knows.
#[derive(Debug, Default, Clone, Component)]
pub struct ThreadRecords {
	/// The actors the window's scene authors, in author order, then those
	/// that authored a stored post.
	actors: Vec<Actor>,
	/// Every post, in [`PostId`] order, each shared with any [`ShownPosts`]
	/// or [`ThreadChanges`] holding it unchanged.
	posts: Vec<Arc<Post>>,
	/// Per-post response metadata, used for `previous_response_id` chaining.
	metas: HashMap<PostId, ResponseMeta>,
	/// How each post last changed, for the readers asking since when.
	log: RevisionLog,
	/// The revision window sync last took its changes at.
	sent: Revision,
	/// The posts holding their file's bytes in memory.
	held: BTreeSet<PostId>,
	/// The posts still in progress.
	in_progress: BTreeSet<PostId>,
	/// Each `edited` or `deleted` log by its id, with its subject.
	changes: BTreeMap<PostId, PostId>,
	/// Each `compaction` post by its id, with its author.
	compactions: BTreeMap<PostId, ActorId>,
	/// The seed's last post ([`Thread::seed`], a branch's its lineage
	/// root's), which a context reads apart from a summary.
	seed: Option<PostId>,
	/// The oldest post of the run when the thread holds older posts the
	/// records lack, `None` when the run reaches back to its first post or
	/// to the seed held before it.
	run_start: Option<PostId>,
}

impl ThreadRecords {
	pub fn new() -> Self { Self::default() }

	// ── actors ──────────────────────────────────────────────────────────

	/// The actors, the scene's in author order first.
	pub fn actors(&self) -> &[Actor] { &self.actors }

	/// Resolve an actor by id, in memory.
	#[track_caller]
	pub fn actor(&self, id: &ActorId) -> Result<&Actor> {
		self.actors
			.iter()
			.find(|actor| actor.id() == id)
			.ok_or_else(|| bevyhow!("No actor {id} in records"))
	}

	/// Insert an actor, replacing one of its id in place.
	pub fn insert_actor(&mut self, actor: Actor) -> ActorId {
		let id = actor.id().clone();
		match self.actors.iter_mut().find(|existing| existing.id() == &id) {
			Some(existing) => *existing = actor,
			None => self.actors.push(actor),
		}
		id
	}

	/// Insert `actor` unless one of its id is already here.
	fn insert_actor_absent(&mut self, actor: &Actor) {
		if self.actor(actor.id()).is_err() {
			self.actors.push(actor.clone());
		}
	}

	/// The author of a post naming `actor` or none: the named actor, else the
	/// first actor of kind User the scene authors.
	///
	/// ## Errors
	///
	/// Errors when the named actor is not in the records, or none is named
	/// and the records hold no User actor.
	pub fn author(&self, actor: Option<&ActorId>) -> Result<ActorId> {
		match actor {
			Some(actor) => self.actor(actor)?.id().clone().xok(),
			None => self
				.actors
				.iter()
				.find(|actor| actor.kind() == ActorKind::User)
				.map(|actor| actor.id().clone())
				.ok_or_else(|| {
					bevyhow!(
						"no actor is named to author the post, and the thread holds no User actor"
					)
				}),
		}
	}

	// ── posts ───────────────────────────────────────────────────────────

	/// Every post, everything included: a deleted post's body and an edit's
	/// original are here, so a person or an agent reads [`Self::shown`].
	pub fn posts(&self) -> &[Arc<Post>] { &self.posts }
	pub fn is_empty(&self) -> bool { self.posts.is_empty() }
	pub fn last_post(&self) -> Option<&Post> {
		self.posts.last().map(|post| &**post)
	}

	/// Append a post, or replace the existing post with the same id, for window
	/// sync to apply to the thread's other windows. New posts keep insertion
	/// (ie [`PostId`]) order; modified posts update in place.
	pub fn upsert_post(&mut self, post: Post) {
		self.log.local(post.id());
		self.upsert_quiet(Arc::new(post));
	}

	/// Upsert `post`, a change this window wrote to the store itself, ie an
	/// edit: window sync carries it and the flush never writes it again.
	pub(crate) fn upsert_stored(&mut self, post: Post) {
		self.log.stored(post.id());
		self.upsert_quiet(Arc::new(post));
	}

	/// Upsert `post` loaded from the store or received from another window,
	/// found by its id and kept in id order.
	fn upsert_received(&mut self, post: Arc<Post>) {
		self.log.received(post.id());
		self.upsert_quiet(post);
	}

	/// Upsert `post` without recording a change.
	fn upsert_quiet(&mut self, post: Arc<Post>) {
		let id = post.id();
		if let Some(subject) = post.subject()
			&& matches!(post.kind(), PostKind::Edited | PostKind::Deleted)
		{
			self.changes.insert(id, subject);
		}
		if post.kind() == &PostKind::Compaction {
			self.compactions.insert(id, post.author().clone());
		}
		match post.holds_bytes() {
			true => self.held.insert(id),
			false => self.held.remove(&id),
		};
		match post.in_progress() {
			true => self.in_progress.insert(id),
			false => self.in_progress.remove(&id),
		};
		match self.index_of(id) {
			Ok(index) => self.posts[index] = post,
			Err(index) => self.posts.insert(index, post),
		}
	}

	/// Where the post `id` is, or would be, in [`PostId`] order.
	fn index_of(&self, id: PostId) -> Result<usize, usize> {
		self.posts.binary_search_by_key(&id, |post| post.id())
	}

	// ── what changed ────────────────────────────────────────────────────

	/// The revision of the newest change, which a reader keeps to ask what
	/// changed since.
	pub fn revision(&self) -> Revision { self.log.revision() }

	/// Each post that changed after `since`, in the order it last changed,
	/// with how: a range read, proportional to the change.
	pub fn changed_since(
		&self,
		since: Revision,
	) -> impl Iterator<Item = (PostId, PostRevision)> {
		self.log.since(since)
	}

	// ── the run ─────────────────────────────────────────────────────────

	/// Whether the thread holds posts older than the run the records lack,
	/// which reading older posts fills in ([`ThreadStore::read_older`]).
	pub fn has_older(&self) -> bool { self.run_start.is_some() }

	/// The oldest post of the run when older posts exist.
	pub fn run_start(&self) -> Option<PostId> { self.run_start }

	/// The newest post held before the run, ie the seed's last, which
	/// reading older posts stops at.
	pub fn run_floor(&self) -> Option<PostId> {
		let start = self.run_start?;
		let index = self.index_of(start).ok()?;
		index.checked_sub(1).map(|index| self.posts[index].id())
	}

	/// The seed's last post, if the thread has a seed: what a context
	/// reads apart from a summary that does not hold it.
	pub fn seed(&self) -> Option<PostId> { self.seed }

	/// Name the seed's last post.
	pub fn set_seed(&mut self, seed: Option<PostId>) { self.seed = seed; }

	/// Each `compaction` post, oldest first, with its author.
	pub(crate) fn compactions(
		&self,
	) -> impl DoubleEndedIterator<Item = (PostId, &ActorId)> {
		self.compactions.iter().map(|(id, author)| (*id, author))
	}

	/// Each `edited` or `deleted` log after `read`, oldest first, with the
	/// post it changed.
	pub(crate) fn changes_after(
		&self,
		read: PostId,
	) -> impl Iterator<Item = (PostId, PostId)> {
		self.changes
			.range((
				core::ops::Bound::Excluded(read),
				core::ops::Bound::Unbounded,
			))
			.map(|(log, subject)| (*log, *subject))
	}

	/// Whether a change to `post` reaches these records: a post they lack
	/// older than the run is one the window has not loaded, so it is dropped.
	fn reaches(&self, post: &Post) -> bool {
		self.run_start.is_none_or(|start| post.id() >= start)
			|| self.post(post.id()).is_some()
	}

	/// Join each post with its authoring [`Actor`] from the records.
	pub fn authored_posts(
		&self,
	) -> impl DoubleEndedIterator<Item = AuthoredPost<'_>> {
		self.posts.iter().filter_map(|post| {
			self.actor(post.author())
				.ok()
				.map(|actor| AuthoredPost { post, actor })
		})
	}

	/// The posts after `since`, in [`PostId`] order, every post for `None`.
	pub fn posts_since(&self, since: Option<PostId>) -> &[Arc<Post>] {
		let start = since.map_or(0, |since| {
			self.posts.partition_point(|post| post.id() <= since)
		});
		&self.posts[start..]
	}

	/// The posts still in progress, oldest first.
	pub fn in_progress(&self) -> impl Iterator<Item = &Post> {
		self.in_progress.iter().filter_map(|id| self.post(*id))
	}

	/// The post `id` joined with its author, if the records hold both.
	pub fn authored(&self, id: PostId) -> Option<AuthoredPost<'_>> {
		let post = self.post(id)?;
		let actor = self.actor(post.author()).ok()?;
		AuthoredPost { post, actor }.xsome()
	}

	/// Every authored post that is not a log: what was said, rather than
	/// what happened.
	pub fn content_posts(&self) -> impl Iterator<Item = AuthoredPost<'_>> {
		self.authored_posts().filter(|view| !view.post.is_log())
	}

	/// The newest post that is not a log.
	pub fn last_content_post(&self) -> Option<&Post> {
		self.posts
			.iter()
			.rev()
			.find(|post| !post.is_log())
			.map(|post| &**post)
	}

	/// The thread's last word: the newest post that is neither a log nor a
	/// `tool_output` closing an interrupted call, which nobody said. A note
	/// that cut a tool call is the last word, though the output closing that
	/// call was written after it.
	pub fn last_word(&self) -> Option<&Post> {
		self.posts
			.iter()
			.rev()
			.find(|post| post.is_word())
			.map(|post| &**post)
	}

	/// What the agent `reader` reads of `shown`, after an exclusive cursor,
	/// in model order (a call followed at once by its output): its context
	/// ([`Self::context_bounds`]), the seed it reads apart, its compaction's
	/// note ([`Self::compaction_note`]) and every post after the
	/// compaction's subject, else the whole of `shown`. Of those, its own
	/// posts and anyone else's that is not a log, another actor's call and
	/// its output as one item, and each run of joins, edits and deletions
	/// between them as one note ([`Self::note`]). An edited post reads as it
	/// now stands; a deleted post, reasoning whoever wrote it and any other
	/// log stay out, so none splits a run.
	pub fn model_posts<'a>(
		&'a self,
		shown: &'a ShownPosts,
		reader: &ActorId,
		after: Option<PostId>,
	) -> Vec<ModelPost<'a>> {
		let bounds = self.context_bounds(shown, reader);
		let ordered = self
			.model_order(shown)
			.into_iter()
			.filter(|view| Self::in_context(view.post, bounds))
			.collect::<Vec<_>>();
		let outputs = ordered
			.iter()
			.filter_map(|view| Some((view.post.subject()?, *view)))
			.filter(|(_, view)| view.post.kind() == &PostKind::ToolOutput)
			.collect::<HashMap<_, _>>();
		// the seed read apart, the compaction's note, then the rest
		let seed = bounds.and_then(|(seed, _)| seed);
		let split = ordered
			.iter()
			.position(|view| seed.is_none_or(|seed| view.post.id() > seed))
			.unwrap_or(ordered.len());
		let split = match bounds {
			Some(_) => split,
			None => 0,
		};
		let mut entries =
			ordered[..split].iter().copied().map(Ok).collect::<Vec<_>>();
		if let Some((_, compaction)) = bounds {
			entries.push(Err(Self::compaction_note(compaction)));
		}
		entries.extend(ordered[split..].iter().copied().map(Ok));
		let start = after
			.and_then(|after| {
				entries.iter().position(|entry| {
					entry.as_ref().is_ok_and(|view| view.post.id() == after)
				})
			})
			.map_or(0, |index| index + 1);
		let mut items = Vec::new();
		let mut noted = Vec::new();
		for entry in entries.into_iter().skip(start) {
			let view = match entry {
				Ok(view) => view,
				Err(note) => {
					self.push_note(&mut items, &mut noted);
					items.push(ModelPost::Note(note));
					continue;
				}
			};
			if view.post.kind().is_noted() {
				noted.push(view);
				continue;
			}
			let Some(content) = view.post.as_agent_post() else {
				continue;
			};
			let foreign = view.post.author() != reader;
			let item = match content {
				_ if !content.is_model_input() => continue,
				AgentPost::FunctionCall(_) if foreign => ModelPost::Call {
					call: view,
					output: outputs.get(&view.post.id()).copied(),
				},
				// part of its call's item
				AgentPost::FunctionCallOutput(output)
					if output.call().is_some_and(|call| {
						self.post(call)
							.is_some_and(|call| call.author() != reader)
					}) =>
				{
					continue;
				}
				_ => ModelPost::Post(view),
			};
			self.push_note(&mut items, &mut noted);
			items.push(item);
		}
		self.push_note(&mut items, &mut noted);
		items
	}

	/// What a request of `reader` sends of `shown`, chained to the stored
	/// response `chained` if any: the posts after the newest that response
	/// read, less that response's own posts, which the provider holds.
	pub fn request_posts<'a>(
		&'a self,
		shown: &'a ShownPosts,
		reader: &ActorId,
		chained: Option<&ResponseMeta>,
	) -> Vec<ModelPost<'a>> {
		let Some(chained) = chained else {
			return self.model_posts(shown, reader, None);
		};
		self.model_posts(shown, reader, chained.read)
			.into_iter()
			.filter(|item| {
				item.post().is_none_or(|post| {
					self.meta(post.post.id()).is_none_or(|meta| {
						meta.response_id != chained.response_id
					})
				})
			})
			.collect()
	}

	/// The newest post a request reads: every post up to it complete, so a
	/// post still streaming, and every post after it, is sent again by the
	/// next chained request.
	pub fn read_through(&self) -> Option<PostId> {
		let Some(first) = self.in_progress.first() else {
			return self.posts.last().map(|post| post.id());
		};
		let index = self.index_of(*first).ok()?;
		index.checked_sub(1).map(|index| self.posts[index].id())
	}

	/// The post `id`, if the records hold it.
	pub fn post(&self, id: PostId) -> Option<&Post> {
		self.shared(id).map(|post| &**post)
	}

	/// The post `id` as the records share it.
	fn shared(&self, id: PostId) -> Option<&Arc<Post>> {
		self.index_of(id).ok().map(|index| &self.posts[index])
	}

	/// Whether its author edited the post `id`: an `edited` log names it.
	pub fn was_edited(&self, id: PostId) -> bool {
		self.changes.range(id..).any(|(log, subject)| {
			*subject == id
				&& self
					.post(*log)
					.is_some_and(|log| log.kind() == &PostKind::Edited)
		})
	}

	/// Whether a post a response read through `read` changed after it was
	/// read: an `edited` or `deleted` log newer than `read` names a post at
	/// or before it, so the provider holds what was taken back.
	pub fn changed_under(&self, read: PostId) -> bool {
		self.changes_after(read).any(|(_, subject)| subject <= read)
	}

	/// The authored posts of `shown` in the order a model reads them: by
	/// [`PostId`], except that a call is followed at once by the output whose
	/// subject it is, so a post landing between the two, ie a person's note
	/// typed mid tool call, is read after the output and never breaks a
	/// provider's pairing of call and output. A call with no output yet holds
	/// nothing back.
	fn model_order<'a>(
		&'a self,
		shown: &'a ShownPosts,
	) -> Vec<AuthoredPost<'a>> {
		let answered = shown
			.iter()
			.filter(|post| post.kind() == &PostKind::ToolOutput)
			.filter_map(|post| post.subject())
			.collect::<HashSet<_>>();
		let mut ordered = Vec::with_capacity(shown.len());
		let mut held = Vec::new();
		let mut pending = HashSet::<PostId>::default();
		for view in shown.iter().filter_map(|post| {
			self.actor(post.author())
				.ok()
				.map(|actor| AuthoredPost { post, actor })
		}) {
			match view.post.kind() {
				PostKind::ToolCall => {
					if answered.contains(&view.post.id()) {
						pending.insert(view.post.id());
					}
					ordered.push(view);
				}
				PostKind::ToolOutput => {
					if let Some(call) = view.post.subject() {
						pending.remove(&call);
					}
					ordered.push(view);
					if pending.is_empty() {
						ordered.append(&mut held);
					}
				}
				_ if pending.is_empty() => ordered.push(view),
				_ => held.push(view),
			}
		}
		ordered.append(&mut held);
		ordered
	}

	/// Push the note of the run `noted` onto `items`, if any, leaving none.
	fn push_note<'a>(
		&self,
		items: &mut Vec<ModelPost<'a>>,
		noted: &mut Vec<AuthoredPost<'a>>,
	) {
		if !noted.is_empty() {
			items.push(ModelPost::Note(self.note(noted)));
			noted.clear();
		}
	}

	/// Merge a stored run under these, as a window opens or reads older
	/// posts ([`ThreadStore::open`], [`ThreadStore::read_older`]): its posts
	/// and metadata as received, the actors these records lack, and its
	/// start as the run's, the stored run reaching back from where these
	/// records' did.
	pub fn merge_stored(&mut self, stored: ThreadRecords) {
		let ThreadRecords {
			actors,
			posts,
			metas,
			run_start,
			seed,
			..
		} = stored;
		if seed.is_some() {
			self.seed = seed;
		}
		for actor in &actors {
			self.insert_actor_absent(actor);
		}
		for post in posts {
			self.upsert_received(post);
		}
		self.metas.extend(metas);
		self.run_start = run_start;
		self.lift_minter();
	}

	/// Mark the run as starting at `start`, older posts existing before it,
	/// or reaching the thread's first post with `None`.
	pub(crate) fn set_run_start(&mut self, start: Option<PostId>) {
		self.run_start = start;
	}

	/// Lift this process's minter past the newest post, so a post minted
	/// after is after every post these records know ([`Tid::after`]).
	fn lift_minter(&self) {
		if let Some(newest) = self.posts.last() {
			PostId::after(newest.id());
		}
	}

	/// Rename the thread of the posts of `from` to `to`, as a thread created
	/// under an id another had taken is minted again.
	#[cfg(feature = "action")]
	pub(crate) fn rename_thread(&mut self, from: ThreadId, to: ThreadId) {
		if from == to {
			return;
		}
		for post in self.posts.iter_mut().filter(|post| post.thread() == from) {
			Arc::make_mut(post).set_thread(to);
		}
	}

	/// Take what this window changed since window sync last took it: each
	/// changed post as it stands, its author and its metadata. Empty when
	/// nothing changed.
	pub fn take_changes(&mut self) -> ThreadChanges {
		let sent = core::mem::replace(&mut self.sent, self.log.revision());
		let ids = self
			.log
			.since(sent)
			.filter(|(_, change)| {
				change.sent_after(sent) || change.meta_after(sent)
			})
			.map(|(id, _)| id)
			.collect::<Vec<_>>();
		self.changes_of(ids)
	}

	/// The posts `ids` as changes, with their authors and metadata.
	fn changes_of(
		&self,
		ids: impl IntoIterator<Item = PostId>,
	) -> ThreadChanges {
		let mut changes = ThreadChanges::default();
		for post in ids.into_iter().filter_map(|id| self.shared(id)) {
			if let Ok(actor) = self.actor(post.author())
				&& !changes.actors.iter().any(|known| known.id() == actor.id())
			{
				changes.actors.push(actor.clone());
			}
			if let Some(meta) = self.metas.get(&post.id()) {
				changes.metas.push(meta.clone());
			}
			changes.posts.push(post.clone());
		}
		changes
	}

	/// Whether this window changed anything since window sync last took it.
	pub fn has_changes(&self) -> bool {
		self.log.since(self.sent).any(|(_, change)| {
			change.sent_after(self.sent) || change.meta_after(self.sent)
		})
	}

	/// Apply `changes`, another window's, as received: an actor these
	/// records lack is added, the rest are replaced, and a post older than
	/// the run that the records lack is dropped, the window not having
	/// loaded it.
	pub fn apply_changes(&mut self, changes: &ThreadChanges) {
		for actor in &changes.actors {
			self.insert_actor_absent(actor);
		}
		for post in &changes.posts {
			if self.reaches(post) {
				self.upsert_received(post.clone());
			}
		}
		for meta in &changes.metas {
			if self.post(meta.post_id).is_some() {
				self.metas.insert(meta.post_id, meta.clone());
			}
		}
		self.lift_minter();
	}

	/// What a window joining this live one takes from it: the posts the
	/// store may not hold yet, those after `loaded`, the newest post the
	/// joining window read from the store, those in progress and those
	/// `synced` has yet to write, the rest being read from the store. A
	/// window with no store gives every post.
	pub fn joining_changes(
		&self,
		loaded: Option<PostId>,
		synced: Option<&SyncedRecords>,
	) -> ThreadChanges {
		let Some(synced) = synced else {
			return ThreadChanges {
				actors: self.actors.clone(),
				posts: self.posts.clone(),
				metas: self.metas.values().cloned().collect(),
			};
		};
		let after = loaded.map_or(0, |loaded| {
			self.posts.partition_point(|post| post.id() <= loaded)
		});
		let mut ids = self.posts[after..]
			.iter()
			.map(|post| post.id())
			.chain(self.in_progress.iter().copied())
			.chain(synced.unwritten(self))
			.collect::<Vec<_>>();
		ids.sort();
		ids.dedup();
		self.changes_of(ids)
	}

	// ── response metadata ───────────────────────────────────────────────

	/// Set the metadata of its post, for window sync to apply and the flush
	/// to write, again where it already did.
	pub fn set_meta(&mut self, meta: ResponseMeta) {
		self.log.meta(meta.post_id);
		self.metas.insert(meta.post_id, meta);
	}
	pub fn meta(&self, post_id: PostId) -> Option<&ResponseMeta> {
		self.metas.get(&post_id)
	}
	pub fn metas(&self) -> impl Iterator<Item = &ResponseMeta> {
		self.metas.values()
	}

	/// The most recent post authored by `agent_id` whose stored [`ResponseMeta`]
	/// matches the provider and model, for `previous_response_id` patterns,
	/// never one whose `read` covers a post changed since
	/// ([`Self::changed_under`]): the provider holds what was taken back, so
	/// a request chains to an older one, else sends everything. A
	/// compaction starts a new chain, so none older than the agent's newest
	/// is chained to, and the provider never carries the old text on.
	pub fn stored_response(
		&self,
		agent_id: &ActorId,
		provider_slug: &str,
		model_slug: &str,
	) -> Option<(PostId, &ResponseMeta)> {
		let floor = self.newest_compaction(agent_id).map(Post::id);
		self.posts
			.iter()
			.rev()
			.take_while(|post| floor.is_none_or(|floor| post.id() > floor))
			.find_map(|post| {
				if post.author() != agent_id || post.is_log() {
					return None;
				}
				let meta = self.metas.get(&post.id())?;
				(meta.provider_slug == provider_slug
					&& meta.model_slug == model_slug
					&& meta.response_stored
					&& meta.read.is_some_and(|read| !self.changed_under(read)))
				.then_some((post.id(), meta))
			})
	}

	// ── bytes ───────────────────────────────────────────────────────────

	/// The file of every post holding one, oldest first, a deleted post's
	/// included: what the window's memory may hold. A context carries or lists
	/// the files of what is shown ([`ShownPosts::files`], [`KeepBlobs`]).
	pub fn files(&self) -> impl DoubleEndedIterator<Item = PostFile> {
		self.posts.iter().flat_map(|post| post.files())
	}

	/// Drop the bytes held in memory of every post but those holding one of
	/// the newest `keep` files, and those `releasable` refuses, ie not yet
	/// written to the blob store. Nothing is marked changed: the row is
	/// untouched.
	pub fn release_bytes(
		&mut self,
		keep: usize,
		releasable: impl Fn(PostId) -> bool,
	) {
		if self.held.len() <= keep {
			return;
		}
		let carried = PostFile::newest(self.files(), keep)
			.into_iter()
			.map(|(post, _)| post)
			.collect::<HashSet<_>>();
		let released = self
			.held
			.iter()
			.copied()
			.filter(|id| !carried.contains(id) && releasable(*id))
			.collect::<Vec<_>>();
		for id in released {
			if let Ok(index) = self.index_of(id) {
				Arc::make_mut(&mut self.posts[index]).release_bytes();
			}
			self.held.remove(&id);
		}
	}

	/// How many posts hold their file's bytes in memory.
	pub fn held_bytes(&self) -> usize { self.held.len() }
}

/// One window's changes to its records, applied to every other window of
/// the same stored thread ([`ThreadRecords::take_changes`]).
#[derive(Debug, Default, Clone)]
pub struct ThreadChanges {
	/// The authors of [`Self::posts`].
	pub actors: Vec<Actor>,
	/// Each post as it stands, a post in progress included.
	pub posts: Vec<Arc<Post>>,
	/// The metadata of [`Self::posts`].
	pub metas: Vec<ResponseMeta>,
}

impl ThreadChanges {
	pub fn is_empty(&self) -> bool { self.posts.is_empty() }
}

// ═══════════════════════════════════════════════════════════════════════
// AuthoredPost
// ═══════════════════════════════════════════════════════════════════════

/// A post joined with its authoring [`Actor`], sourced from a [`ThreadRecords`].
#[derive(Debug, Clone, Copy)]
pub struct AuthoredPost<'a> {
	pub post: &'a Post,
	pub actor: &'a Actor,
}

impl std::ops::Deref for AuthoredPost<'_> {
	type Target = Post;
	fn deref(&self) -> &Self::Target { self.post }
}

impl AuthoredPost<'_> {
	pub fn actor_id(&self) -> &ActorId { self.actor.id() }

	/// Wraps text in XML metadata tags so models can distinguish speakers.
	/// Used for non-assistant, non-system, non-developer messages.
	pub fn wrap_user_text(&self, text: &str) -> String {
		format!(
			"<post author={} author_kind={} author_id={}>{}</post>",
			self.actor.name(),
			self.actor.kind().input_str(),
			self.actor.id(),
			text
		)
	}

	/// The text of this call and its `output`, the text its answer reads
	/// as ([`RequestBlobs::output_text`]), as another actor's call reads to
	/// an agent that did not make it: one function, so no mapper chooses it.
	pub fn call_text(&self, output: Option<String>) -> String {
		let call = match self.post.as_agent_post() {
			Some(AgentPost::FunctionCall(call)) => call,
			_ => return self.post.to_string(),
		};
		let mut text =
			format!("called `{}` with {}", call.name(), call.arguments());
		match output {
			Some(output) => {
				text.push_str(&format!(", which answered {output}"));
			}
			None => text.push_str(", not yet answered"),
		}
		self.wrap_user_text(&text)
	}
}

/// One item of a model's input ([`ThreadRecords::model_posts`]).
#[derive(Debug, Clone)]
pub enum ModelPost<'a> {
	/// A post that is not a log, sent in the role its author has to the
	/// reading agent.
	Post(AuthoredPost<'a>),
	/// The note of a run of joins, edits and deletions
	/// ([`ThreadRecords::note`]), sent as it is in the user role whoever it
	/// is about, the reading agent included: something that happened, never
	/// something the agent said.
	Note(String),
	/// Another actor's call and its output, if answered: one message in the
	/// caller's role ([`AuthoredPost::call_text`]), since the reader never
	/// made the call.
	Call {
		call: AuthoredPost<'a>,
		output: Option<AuthoredPost<'a>>,
	},
}

impl<'a> ModelPost<'a> {
	/// The post this item is, a note being none.
	pub fn post(&self) -> Option<AuthoredPost<'a>> {
		match self {
			Self::Post(post) => Some(*post),
			Self::Note(_) | Self::Call { .. } => None,
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// ActorRef
// ═══════════════════════════════════════════════════════════════════════

/// A behavior entity's reference to the [`Actor`] it acts as. The actor's full
/// definition lives in the [`ThreadRecords`]; the behavior scene carries only
/// this id plus the actor's behavior (its [`Action`] and tools).
///
/// Produced by the author-to-behavior reduction, which swaps every in-place
/// `Actor` carrying behavior for an `ActorRef`.
#[derive(Debug, Clone, Deref, Component, Reflect)]
#[reflect(Component)]
pub struct ActorRef(pub ActorId);

// ═══════════════════════════════════════════════════════════════════════
// SeedPost
// ═══════════════════════════════════════════════════════════════════════

/// An author-time seed post: an unresolved [`IntoPost`] payload spawned as a
/// child of its `<CreateActor>`, awaiting reduction.
///
/// This is the runtime component the author-to-behavior reduction queries
/// ([`ThreadRecords::reduce`]): it resolves the seed's author (its actor parent)
/// and thread, hoists the materialized [`Post`] into the [`ThreadRecords`], and
/// despawns the entity. An entry continuing its thread despawns it unread
/// ([`Thread::enter`]).
///
/// It is the payload, not the tag. `<CreatePost>` is the `#[template]` that
/// produces a `SeedPost` from markup; the two cannot be merged: a `#[template]`
/// expands away at build and leaves no component for the runtime `Query<&SeedPost>`
/// to read, and `content: IntoPost` is not attribute-coercible so `SeedPost`
/// itself cannot be a tag. The template adapts a coercible `text` into `IntoPost`;
/// this component is what the world stores and reduces. (Mirrors the
/// `<CreateActor>`/[`Actor`] tag/payload pair.) Authored via [`Post::spawn`] /
/// `<CreatePost>`.
#[derive(Debug, Clone, Component)]
pub struct SeedPost {
	pub content: IntoPost,
	/// The kind of a text seed, ie a refusal, its content's own by default.
	pub kind: Option<PostKind>,
	pub status: PostStatus,
}

impl SeedPost {
	/// Resolve into a [`Post`] for the given author and thread.
	pub fn into_post(self, author: ActorId, thread: ThreadId) -> Post {
		match (self.kind, self.content) {
			(Some(kind), IntoPost::Text(text)) => Post::new(
				author,
				thread,
				kind,
				self.status,
				[(SmolStr::from("text"), Value::from(text))]
					.into_iter()
					.collect(),
			),
			(_, content) => content.into_post(author, thread, self.status),
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// The reduction: author scene -> ThreadRecords + behavior scene
// ═══════════════════════════════════════════════════════════════════════

impl ThreadRecords {
	/// Reduce every freshly-spawned authored [`Thread`] into a [`ThreadRecords`] plus
	/// a lean behavior scene, keeping its seed posts.
	///
	/// For each thread lacking records, walk its descendant actors in author order
	/// and: hoist each [`Actor`] into `records.actors`, hoist each [`SeedPost`] into
	/// `records.posts`, then rewrite the actor entity. An actor that carries behavior
	/// (an [`Action`]) becomes an [`ActorRef`] keeping its behavior and tools; a
	/// seed-only actor (eg the system prompt) is despawned, leaving no turn behind.
	///
	/// A thread mounting a store ([`MountThreadStore`]) or holding a
	/// [`SeedSource`] waits for its entry, the one place that knows whether
	/// its seed is kept ([`Thread::enter`]).
	///
	/// Idempotent: gated on `Without<ThreadRecords>`, so it runs once per thread.
	pub fn reduce(
		mut commands: Commands,
		threads: Query<
			(Entity, &Thread),
			(Without<ThreadRecords>, Without<MountThreadStore>),
		>,
		reduction: Reduction,
		#[cfg(feature = "action")] sources: Query<(), With<SeedSource>>,
	) -> Result {
		for (entity, thread) in threads.iter() {
			#[cfg(feature = "action")]
			if reduction
				.children
				.iter_descendants(entity)
				.any(|child| sources.contains(child))
			{
				continue;
			}
			reduction.reduce(&mut commands, entity, thread.id(), true)?;
		}
		Ok(())
	}

	/// Reduce the thread at `entity` as its entry decides, consuming its
	/// [`MountThreadStore`]: keeping its seed posts when the entry creates
	/// its thread, else despawning them and every [`SeedSource`] unread. A
	/// thread already reduced, which mounts no store, is left as it is.
	#[cfg(feature = "action")]
	pub(crate) fn reduce_entered(
		world: &mut World,
		entity: Entity,
		keep_seeds: bool,
	) -> Result {
		world.entity_mut(entity).remove::<MountThreadStore>();
		if world.get::<ThreadRecords>(entity).is_some() {
			return Ok(());
		}
		world.run_system_cached_with::<_, Result, _, _>(
			Self::reduce_one,
			(entity, keep_seeds),
		)??;
		world.flush();
		Ok(())
	}

	/// Reduce the one thread at `entity`, keeping or dropping its seeds.
	#[cfg(feature = "action")]
	fn reduce_one(
		In((entity, keep_seeds)): In<(Entity, bool)>,
		mut commands: Commands,
		threads: Query<&Thread>,
		reduction: Reduction,
	) -> Result {
		let thread = threads.get(entity)?.id();
		reduction.reduce(&mut commands, entity, thread, keep_seeds)
	}

	/// Run [`ThreadRecords::reduce`] immediately and flush, for manual consumers that
	/// pump the world directly (eg `Post::run_oneshot`, tests) rather than the schedule.
	pub fn reduce_now(world: &mut World) {
		let _ = world.run_system_cached::<Result, _, _>(Self::reduce);
		world.flush();
	}

	/// Spawn an authored thread `scene` and reduce it into a [`ThreadRecords`] plus
	/// behavior scene in one step, returning the root entity. The behavior is *not*
	/// triggered; the caller drives it (eg `entity.call(())`), so the records are
	/// always populated before any turn runs.
	pub async fn spawn_reduced(
		world: AsyncWorld,
		scene: impl Bundle,
	) -> Result<Entity> {
		world
			.with(move |world: &mut World| {
				let root = world.spawn(scene).id();
				Self::reduce_now(world);
				root
			})
			.await
			.xok()
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Reduction: the walk over one authored thread
// ═══════════════════════════════════════════════════════════════════════

/// The queries reducing one authored thread ([`ThreadRecords::reduce`]).
#[derive(SystemParam)]
pub struct Reduction<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	actors: Query<'w, 's, &'static Actor>,
	seeds: Query<'w, 's, &'static SeedPost>,
	#[cfg(feature = "action")]
	sources: Query<'w, 's, (), With<SeedSource>>,
	#[cfg(feature = "action")]
	behaviors: Query<'w, 's, (), With<ActionMeta>>,
}

impl Reduction<'_, '_> {
	/// Reduce the thread at `entity`, its posts belonging to `thread`,
	/// keeping or dropping its seed posts.
	fn reduce(
		&self,
		commands: &mut Commands,
		entity: Entity,
		thread: ThreadId,
		keep_seeds: bool,
	) -> Result {
		let mut records = ThreadRecords::new();

		// descendant actor entities, in author order
		let actor_entities = self
			.children
			.iter_descendants_depth_first(entity)
			.filter(|child| self.actors.contains(*child))
			.collect::<Vec<_>>();

		for actor_entity in actor_entities {
			let actor = self.actors.get(actor_entity)?.clone();
			// two actors of one id would share every post and turn
			if let Ok(existing) = records.actor(actor.id()) {
				bevybail!(
					"the actors `{}` and `{}` share the id `{}`: give one an id of its own, ie `<CreateActor name=\"{}\" id=\"..\"/>`",
					existing.name(),
					actor.name(),
					actor.id(),
					actor.name()
				);
			}
			let actor_id = records.insert_actor(actor);

			// hoist or drop this actor's seed posts, despawning their entities.
			// Scan all descendants (not just direct children) so seeds authored
			// through a markup `<slot/>` are still found; actors never nest, so
			// every seed under an actor entity is unambiguously its own.
			for child in
				self.children.iter_descendants_depth_first(actor_entity)
			{
				if let Ok(seed_post) = self.seeds.get(child) {
					if keep_seeds {
						records.upsert_post(
							seed_post
								.clone()
								.into_post(actor_id.clone(), thread),
						);
					}
					commands.entity(child).despawn();
				}
				#[cfg(feature = "action")]
				if !keep_seeds && self.sources.contains(child) {
					commands.entity(child).despawn();
				}
			}

			// rewrite the actor: behavior -> ActorRef, seed-only -> despawn
			#[cfg(feature = "action")]
			let has_behavior = self.behaviors.contains(actor_entity);
			#[cfg(not(feature = "action"))]
			let has_behavior = true;
			if has_behavior {
				commands
					.entity(actor_entity)
					.remove::<Actor>()
					.insert(ActorRef(actor_id.clone()));
			} else {
				commands.entity(actor_entity).despawn();
			}
		}

		// a kept seed is every post the records hold before anyone joins
		if keep_seeds {
			records.set_seed(records.last_post().map(Post::id));
		}
		commands.entity(entity).insert(records);
		Ok(())
	}
}

// ═══════════════════════════════════════════════════════════════════════
// ThreadRecordsQuery: mutate the thread records from a behavior entity
// ═══════════════════════════════════════════════════════════════════════

/// Locate and mutate the [`ThreadRecords`] of the window a behavior entity
/// belongs to. Behavior entities ([`ActorRef`]) are descendants of their
/// window; the records live on the window.
#[derive(SystemParam)]
pub struct ThreadRecordsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	actor_refs: Query<'w, 's, &'static ActorRef>,
	thread_records:
		Query<'w, 's, (&'static Thread, &'static mut ThreadRecords)>,
}

impl ThreadRecordsQuery<'_, '_> {
	/// Walk ancestors to the window carrying the [`ThreadRecords`].
	pub fn thread_entity(&self, entity: Entity) -> Result<Entity> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|ancestor| self.thread_records.contains(*ancestor))
			.ok_or_else(|| {
				bevyhow!("No ThreadRecords in ancestors of {entity}")
			})
	}

	/// The [`ActorId`] a behavior entity acts as.
	pub fn actor_id(&self, entity: Entity) -> Result<ActorId> {
		self.actor_refs
			.get(entity)
			.map(|actor_ref| actor_ref.0.clone())
			.map_err(|_| bevyhow!("entity {entity} has no ActorRef"))
	}

	/// The [`ThreadId`] of the thread a behavior entity belongs to.
	pub fn thread_id(&self, entity: Entity) -> Result<ThreadId> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get(thread_entity)?.0.id())
	}

	/// The window's records.
	pub fn records(&self, entity: Entity) -> Result<&ThreadRecords> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get(thread_entity)?.1)
	}

	/// Mutable access to the window's records.
	pub fn records_mut(
		&mut self,
		entity: Entity,
	) -> Result<Mut<'_, ThreadRecords>> {
		let thread_entity = self.thread_entity(entity)?;
		Ok(self.thread_records.get_mut(thread_entity)?.1)
	}

	/// Append a post to the window's records.
	pub fn push_post(&mut self, entity: Entity, post: Post) -> Result {
		self.records_mut(entity)?.upsert_post(post);
		Ok(())
	}

	/// Record a response's `usage` on the meta of `post`, its last post,
	/// which the next flush writes again even if it already went.
	pub fn set_usage(
		&mut self,
		entity: Entity,
		post: PostId,
		usage: Usage,
	) -> Result {
		let mut records = self.records_mut(entity)?;
		let Some(mut meta) = records.meta(post).cloned() else {
			return Ok(());
		};
		meta.usage = Some(usage);
		records.set_meta(meta);
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The reduction hoists every actor and seed post into the records, despawns
	/// seed-only actors, and rewrites behavior actors to id-referencing entities.
	#[beet_core::test]
	fn reduction_hoists_and_rewrites() {
		let mut world = World::new();
		let thread = world
			.spawn((Thread::default(), children![
				// seed-only actor: hoisted then despawned
				(Actor::system(), children![Post::spawn("sys prompt")]),
				// behavior actor: kept as an ActorRef
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		world.flush();
		ThreadRecords::reduce_now(&mut world);

		// the records hold both actors and the one seed post, author resolvable
		let records = world.get::<ThreadRecords>(thread).unwrap();
		records.actors().len().xpect_eq(2);
		records.posts().len().xpect_eq(1);
		records.posts()[0]
			.to_string()
			.xpect_eq("sys prompt".to_string());
		let author = records.posts()[0].author();
		records
			.actor(author)
			.unwrap()
			.kind()
			.xpect_eq(ActorKind::System);

		// behavior scene: exactly one ActorRef (the agent), no Actor or SeedPost
		// entities survive, and the survivor keeps its streamer behavior
		world.query::<&ActorRef>().iter(&world).count().xpect_eq(1);
		world.query::<&Actor>().iter(&world).count().xpect_eq(0);
		world.query::<&SeedPost>().iter(&world).count().xpect_eq(0);
		world
			.query_filtered::<Entity, (With<ActorRef>, With<MockPostStreamer>)>(
			)
			.iter(&world)
			.count()
			.xpect_eq(1);
	}

	/// Two actors of one id in a scene are refused, naming both.
	#[beet_core::test]
	fn refuses_two_actors_of_one_id() {
		let mut world = World::new();
		world.spawn((Thread::default(), children![
			Actor::new("Ann", ActorKind::User).unwrap(),
			Actor::new_with_id(
				actor_id!("ann"),
				"Annie",
				ActorKind::User,
			),
		]));
		world
			.run_system_cached::<Result, _, _>(ThreadRecords::reduce)
			.unwrap()
			.unwrap_err()
			.to_string()
			.xpect_contains("`Ann` and `Annie` share the id `ann`");
	}

	/// Streaming-shaped records mutation: upsert appends new posts and replaces
	/// existing ones in place, ordered by id.
	#[beet_core::test]
	fn upsert_appends_then_replaces() {
		let thread = ThreadId::now();
		let author = actor_id!("author");
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::new_with_id(
			author.clone(),
			"A",
			ActorKind::Agent,
		));

		let mut post =
			AgentPost::new_text(author, thread, "hel", PostStatus::InProgress);
		records.upsert_post(post.clone());
		records.posts().len().xpect_eq(1);

		// same id streamed further: replaced in place, not appended
		post.set_text("hello");
		post.set_status(PostStatus::Completed);
		records.upsert_post(post);
		records.posts().len().xpect_eq(1);
		records
			.last_post()
			.unwrap()
			.to_string()
			.xpect_eq("hello".to_string());
		records.last_post().unwrap().in_progress().xpect_false();
	}

	/// A function call is followed at once by its output in what a model
	/// reads: a post and a join landing between them are read after the
	/// output, a call without an output holds nothing back, and a cursor
	/// reads on from its place in that order.
	#[beet_core::test]
	fn model_order_pairs_each_call_with_its_output() {
		let thread = ThreadId::now();
		let person = actor_id!("person");
		let agent = actor_id!("agent");
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::new_with_id(
			person.clone(),
			"Cam",
			ActorKind::User,
		));
		records.insert_actor(Actor::new_with_id(
			agent.clone(),
			"Bot",
			ActorKind::Agent,
		));
		let text = |author: &ActorId, text: &str| {
			AgentPost::new_text(
				author.clone(),
				thread,
				text,
				PostStatus::Completed,
			)
		};
		let call = |call_id: &str| {
			AgentPost::new_function_call(
				agent.clone(),
				thread,
				"wait",
				call_id,
				"{}",
				PostStatus::Completed,
			)
		};
		let output = |call: &Post| {
			AgentPost::new_function_call_output(
				agent.clone(),
				thread,
				call.id(),
				call.field("call_id").unwrap_or_default(),
				"done",
				PostStatus::Completed,
			)
		};
		// each minted in turn, so ids follow the conversation
		let go = text(&person, "go");
		let first_call = call("first");
		let cursor = first_call.id();
		for post in [
			go,
			first_call.clone(),
			text(&person, "by the way"),
			Post::new_log(person.clone(), thread, &SessionOpened::default())
				.unwrap(),
			output(&first_call),
			call("unanswered"),
			text(&person, "still there?"),
		] {
			records.upsert_post(post);
		}
		let read = |after| {
			records
				.model_posts(&records.shown(..), &agent, after)
				.into_iter()
				.map(|item| match item {
					ModelPost::Note(_) => "note".to_string(),
					ModelPost::Call { .. } => "another's call".to_string(),
					ModelPost::Post(view) => match AgentPost::new(view.post) {
						Some(AgentPost::FunctionCall(_)) => "call".into(),
						Some(AgentPost::FunctionCallOutput(_)) => {
							"output".into()
						}
						_ => view.post.to_string(),
					},
				})
				.collect::<Vec<_>>()
		};
		read(None).xpect_eq(
			[
				"go",
				"call",
				"output",
				"by the way",
				"note",
				"call",
				"still there?",
			]
			.map(String::from)
			.to_vec(),
		);
		read(Some(cursor)).xpect_eq(
			["output", "by the way", "note", "call", "still there?"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// Another actor's call and its output are one item, reasoning stays out
	/// whoever wrote it, and the reader's own call is its own.
	#[beet_core::test]
	fn another_actors_call_is_one_item() {
		let thread = ThreadId::now();
		let mut records = ThreadRecords::new();
		for actor in [Actor::agent(), Actor::user()] {
			records.insert_actor(actor);
		}
		let other = Actor::new("Other", ActorKind::Agent).unwrap();
		records.insert_actor(other.clone());
		let call = AgentPost::new_function_call(
			other.id().clone(),
			thread,
			"search",
			"c1",
			"{}",
			PostStatus::Completed,
		);
		for post in [
			AgentPost::new_reasoning(
				other.id().clone(),
				thread,
				"thinking",
				PostStatus::Completed,
			),
			call.clone(),
			AgentPost::new_function_call_output(
				other.id().clone(),
				thread,
				call.id(),
				"c1",
				"found",
				PostStatus::Completed,
			),
			AgentPost::new_reasoning(
				ActorId::AGENT,
				thread,
				"mine",
				PostStatus::Completed,
			),
		] {
			records.upsert_post(post);
		}
		let shown = records.shown(..);
		let items = records.model_posts(&shown, &ActorId::AGENT, None);
		items.len().xpect_eq(1);
		let ModelPost::Call { call, output } = &items[0] else {
			panic!("expected a call, found {:?}", items[0]);
		};
		call.call_text(
			output
				.as_ref()
				.map(|output| RequestBlobs::default().output_text(output)),
		)
		.xpect_eq(
			"<post author=Other author_kind=agent author_id=other>called `search` with {}, which answered found</post>",
		);
		// the caller reads its own call as a call
		records
			.model_posts(&shown, other.id(), None)
			.into_iter()
			.filter(|item| matches!(item, ModelPost::Post(_)))
			.count()
			.xpect_eq(2);
	}

	/// A post's author is the named actor, else the first User actor the
	/// scene authors, and with neither there is none.
	#[beet_core::test]
	fn author_falls_back_to_the_first_user() {
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::agent());
		records
			.author(None)
			.unwrap_err()
			.to_string()
			.xpect_contains("holds no User actor");
		let second = Actor::new("Zed", ActorKind::User).unwrap();
		records.insert_actor(second.clone());
		records.insert_actor(Actor::new("Ann", ActorKind::User).unwrap());
		// authored first, though `ann` sorts before `zed`
		records.author(None).unwrap().xpect_eq(second.id().clone());
		records
			.author(Some(&ActorId::AGENT))
			.unwrap()
			.xpect_eq(ActorId::AGENT);
		records
			.author(Some(&actor_id!("nobody")))
			.unwrap_err();
	}
}
