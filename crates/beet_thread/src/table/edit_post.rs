//! Editing and deleting a post, see [`EditPost`] and [`DeletePost`].
//!
//! Only its author may change a post, and only a completed `text` or `media`
//! post of the thread itself, never one a branch inherits (the model page,
//! [`crate::table`], "Editing and deleting"). Each change is one atomic batch
//! in the post's partition: the post put under the cid the window read it
//! at, and its log, an [`PostEdited`] or [`PostDeleted`], absent. A
//! conflicting change is answered to whoever asked, never retried over
//! another's. Nothing is scrubbed: the records and the store keep the
//! original, and only what is shown ([`ShownPosts`]) leaves it out.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The log of a post edited by its author, its subject the post and its body
/// the original's. A post edited twice has two, so its whole history reads
/// by the `by_subject` index.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostEdited {
	/// The post's body before this edit.
	pub was: Map,
}

impl LogKind for PostEdited {
	const KIND: &'static str = "edited";
}

impl PostEdited {
	/// The line of an edit's or a deletion's note: who made it and when the
	/// post it changed was made ([`ThreadRecords::note`]).
	pub(crate) fn line(log: &AuthoredPost) -> String {
		let verb = match log.post.kind() {
			PostKind::Deleted => "deleted",
			_ => "edited",
		};
		let made = log
			.post
			.subject()
			.map(|subject| {
				format!(" from {}", subject.created().format_iso8601_mins())
			})
			.unwrap_or_default();
		format!("{} {verb} a post{made}.", log.actor.name())
	}
}

/// The log of a post deleted by its author, its subject the post, which
/// stays as a tombstone of status `deleted` keeping its body.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostDeleted {}

impl LogKind for PostDeleted {
	const KIND: &'static str = "deleted";
}

/// Replace the body of a post with `content`, as its author: one atomic
/// batch putting the post under the cid its window read it at and appending
/// a [`PostEdited`] log holding the original.
///
/// ```no_run
/// # use beet_thread::prelude::*;
/// # use beet_core::prelude::*;
/// # async fn edit(window: AsyncEntity, post: PostId) -> Result {
/// EditPost::new(ActorId::USER, post, "hello, corrected")
/// 	.apply(&window)
/// 	.await
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct EditPost {
	/// The actor asking, who must be the post's author.
	pub actor: ActorId,
	/// The post edited.
	pub post: PostId,
	/// What the post now holds, of the post's own kind.
	pub content: IntoPost,
}

impl EditPost {
	pub fn new(
		actor: ActorId,
		post: PostId,
		content: impl Into<IntoPost>,
	) -> Self {
		Self {
			actor,
			post,
			content: content.into(),
		}
	}

	/// Edit the post in the window `window`'s thread, and its records, which
	/// window sync carries to the thread's other windows.
	///
	/// ## Errors
	///
	/// Errors when the change is refused (Editing and deleting), or another
	/// change to the post landed since the window read it.
	pub async fn apply(self, window: &AsyncEntity) -> Result {
		PostChange::Edit(self.content)
			.apply(window, self.actor, self.post)
			.await
	}
}

/// Delete a post, as its author: one atomic batch putting the post under the
/// cid its window read it at with the status `deleted`, its body kept, and
/// appending a [`PostDeleted`] log.
#[derive(Debug, Clone)]
pub struct DeletePost {
	/// The actor asking, who must be the post's author.
	pub actor: ActorId,
	/// The post deleted.
	pub post: PostId,
}

impl DeletePost {
	pub fn new(actor: ActorId, post: PostId) -> Self { Self { actor, post } }

	/// Delete the post in the window `window`'s thread, and its records,
	/// which window sync carries to the thread's other windows.
	///
	/// ## Errors
	///
	/// As [`EditPost::apply`].
	pub async fn apply(self, window: &AsyncEntity) -> Result {
		PostChange::Delete
			.apply(window, self.actor, self.post)
			.await
	}
}

/// A change an author makes to their own post.
enum PostChange {
	Edit(IntoPost),
	Delete,
}

impl PostChange {
	/// Make this change to `post` as `actor` in the window `window`: flush
	/// the window so the store holds the post, check and build the change
	/// from the records, write it in one batch, then apply it to the records
	/// as written. A window with no store changes its records the same way.
	async fn apply(
		self,
		window: &AsyncEntity,
		actor: ActorId,
		post: PostId,
	) -> Result {
		ThreadStore::flush(window).await?;
		let id = window.id();
		let (store, read, changed) = window
			.world()
			.with(move |world: &mut World| -> Result<_> {
				let thread = world
					.get::<Thread>(id)
					.ok_or_else(|| bevyhow!("the entity {id} is no window"))?;
				let records =
					world.get::<ThreadRecords>(id).ok_or_else(|| {
						bevyhow!(
							"the window {id} holds no records, has it been entered?"
						)
					})?;
				let read = records.post(post).cloned().ok_or_else(|| {
					bevyhow!("the window {id} has not loaded the post {post}")
				})?;
				let changed = self.build(thread, &actor, &read)?;
				let store = world.get::<ThreadStore>(id).cloned();
				(store, read, changed).xok()
			})
			.await?;
		if let Some(store) = &store {
			store.change(&read, &changed).await?;
		}
		window
			.world()
			.with(move |world: &mut World| -> Result {
				let mut records =
					world.get_mut::<ThreadRecords>(id).ok_or_else(|| {
						bevyhow!("the window {id} closed during the change")
					})?;
				// with no store, nothing else refuses a change landing between
				if store.is_none() && records.post(post) != Some(&read) {
					return Err(PostChange::conflict(post));
				}
				let [changed, log] = changed;
				records.upsert_stored(changed);
				records.upsert_stored(log);
				Ok(())
			})
			.await
	}

	/// The post changed and its log, refused as [`Post::check_change`]
	/// refuses.
	fn build(
		self,
		thread: &Thread,
		actor: &ActorId,
		read: &Post,
	) -> Result<[Post; 2]> {
		let id = read.id();
		read.check_change(actor, thread.id())?;
		let mut changed = read.clone();
		let log = match self {
			Self::Edit(content) => {
				let content = content.into_post(
					actor.clone(),
					thread.id(),
					read.status(),
				);
				if content.kind() != read.kind() {
					bevybail!(
						"the post {id} is a `{}` post, so it is not edited into a `{}`",
						read.kind(),
						content.kind()
					);
				}
				changed.set_body(content.body().clone());
				changed.set_bytes(
					(0..content.files().len())
						.filter_map(|index| content.file_bytes(index).cloned())
						.collect(),
				);
				Post::new_log(actor.clone(), thread.id(), &PostEdited {
					was: read.body().clone(),
				})?
			}
			Self::Delete => {
				changed.set_status(PostStatus::Deleted);
				Post::new_log(actor.clone(), thread.id(), &PostDeleted {})?
			}
		};
		[changed, log.with_subject(id)].xok()
	}

	/// The error answering a change to `post` that another landed before.
	pub(crate) fn conflict(post: PostId) -> BevyError {
		HttpError::new(
			StatusCode::CONFLICT,
			format!(
				"the post {post} changed since it was read, so this change is refused: read it again and retry"
			),
		)
		.into()
	}
}

impl Post {
	/// Refuse a change to this post by `actor` in the thread `thread`, unless
	/// `actor` authored it and it is a completed `text` or `media` post of
	/// that thread itself, never one a branch inherits.
	pub fn check_change(&self, actor: &ActorId, thread: ThreadId) -> Result {
		let id = self.id();
		if self.author() != actor {
			bevybail!(
				"`{actor}` may not change the post {id}: only its author `{}` may",
				self.author()
			);
		}
		if self.thread() != thread {
			bevybail!(
				"the post {id} is inherited from the thread {} this one branches from, so it is not changed here",
				self.thread()
			);
		}
		if !matches!(self.kind(), PostKind::Text | PostKind::Media) {
			bevybail!(
				"the post {id} is a `{}` post: only a `text` or `media` post is changed",
				self.kind()
			);
		}
		if self.status() != PostStatus::Completed {
			bevybail!(
				"the post {id} is {}: only a completed post is changed",
				self.status().as_str()
			);
		}
		Ok(())
	}
}

impl ThreadStore {
	/// Write `changed`, the post `read` as changed, and its log as one batch
	/// in the post's partition: the post under the cid `read` has, the log
	/// absent, any bytes `changed` holds first.
	///
	/// ## Errors
	///
	/// Errors with a `409` when the stored post is no longer `read`.
	pub(crate) async fn change(
		&self,
		read: &Post,
		[changed, log]: &[Post; 2],
	) -> Result {
		let partition = self.partition(read.thread())?;
		let posts = partition.table::<Post>();
		let cid = posts.stored(read.clone())?.cid().clone();
		let writes = vec![
			posts.put_write(changed.clone(), Precondition::Matches(cid))?,
			posts.put_write(log.clone(), Precondition::Absent)?,
		];
		self.write_blobs(core::slice::from_ref(changed)).await?;
		match partition.apply(writes).await {
			Err(err)
				if matches!(
					TableError::of(&err),
					Some(TableError::Conflict { .. })
				) =>
			{
				Err(PostChange::conflict(read.id()))
			}
			result => result,
		}
	}
}

#[cfg(all(test, feature = "action"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use std::sync::Arc;

	const ANN: ActorId = ActorId::from_static("ann");
	const BOB: ActorId = ActorId::from_static("bob");

	/// An app over a table store at `root`.
	fn app() -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let root = app
			.world_mut()
			.spawn(TableStore::new(InMemoryTableStore::new()))
			.id();
		(app, root)
	}

	/// A window of Ann and Bob under `root` mounting `threads`, entered with
	/// `params`.
	async fn window(
		app: &mut App,
		root: Entity,
		params: &[(&str, &str)],
	) -> Result<Entity> {
		let window = app
			.world_mut()
			.spawn((
				ChildOf(root),
				Thread::default(),
				MountThreadStore::new("threads"),
				children![
					Actor::new("Ann", ActorKind::User).unwrap(),
					Actor::new("Bob", ActorKind::User).unwrap(),
				],
			))
			.id();
		let request = params
			.iter()
			.fold(Request::get("chat"), |request, (key, value)| {
				request.with_param(key, value)
			});
		let entry = ThreadEntry::parse(request.parts().clone())?;
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				Thread::enter(world, window, entry).await
			})
			.await?;
		window.xok()
	}

	/// Post `text` as `author` in `window`, answering it.
	fn say(app: &mut App, window: Entity, author: ActorId, text: &str) -> Post {
		let thread = app.world().get::<Thread>(window).unwrap().id();
		let post =
			AgentPost::new_text(author, thread, text, PostStatus::Completed);
		app.world_mut()
			.get_mut::<ThreadRecords>(window)
			.unwrap()
			.upsert_post(post.clone());
		app.update();
		post
	}

	/// Edit `post` in `window` into `text` as `actor`.
	async fn edit(
		app: &mut App,
		window: Entity,
		actor: ActorId,
		post: PostId,
		text: &str,
	) -> Result {
		let text = text.to_string();
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				EditPost::new(actor, post, text)
					.apply(&world.entity(window))
					.await
			})
			.await
	}

	/// Delete `post` in `window` as `actor`.
	async fn delete(
		app: &mut App,
		window: Entity,
		actor: ActorId,
		post: PostId,
	) -> Result {
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				DeletePost::new(actor, post)
					.apply(&world.entity(window))
					.await
			})
			.await
	}

	/// The store of `window` and the posts of its thread as stored, once
	/// every flush has landed.
	async fn stored(app: &mut App, window: Entity) -> (ThreadStore, Vec<Post>) {
		app.update();
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				ThreadStore::flush(&world.entity(window)).await
			})
			.await
			.unwrap();
		let store = app.world().get::<ThreadStore>(window).unwrap().clone();
		let thread = app.world().get::<Thread>(window).unwrap().id();
		let posts = store.posts(thread).await.unwrap();
		(store, posts)
	}

	fn records(app: &App, window: Entity) -> &ThreadRecords {
		app.world().get::<ThreadRecords>(window).unwrap()
	}

	/// Only its author changes a post, and only a completed `text` or
	/// `media` post of the thread itself.
	#[beet_core::test]
	async fn refuses_what_is_not_its_authors_to_change() {
		let (mut app, root) = app();
		let window = window(&mut app, root, &[]).await.unwrap();
		let said = say(&mut app, window, ANN, "hi");
		edit(&mut app, window, BOB, said.id(), "hijacked")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("only its author `ann` may");
		// a join is a log
		let join = records(&app, window).posts()[0].id();
		delete(&mut app, window, ANN, join)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("only a `text` or `media` post");
		// a post still streaming
		let thread = app.world().get::<Thread>(window).unwrap().id();
		let streaming =
			AgentPost::new_text(ANN, thread, "typ", PostStatus::InProgress);
		app.world_mut()
			.get_mut::<ThreadRecords>(window)
			.unwrap()
			.upsert_post(streaming.clone());
		edit(&mut app, window, ANN, streaming.id(), "typed")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("is in_progress: only a completed post");
		// a deleted post is changed no further
		delete(&mut app, window, ANN, said.id()).await.unwrap();
		edit(&mut app, window, ANN, said.id(), "back")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("is deleted");
		// a media post is edited only into media
		let photo = AgentPost::new_media(
			ANN,
			thread,
			MediaType::Png,
			vec![1, 2, 3],
			None,
			PostStatus::Completed,
		);
		app.world_mut()
			.get_mut::<ThreadRecords>(window)
			.unwrap()
			.upsert_post(photo.clone());
		edit(&mut app, window, ANN, photo.id(), "words")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("not edited into a `text`");
		// a branch's inherited post is its parent's
		let point = PostRef::new(thread, photo.id());
		let branch = super::test::window(&mut app, root, &[(
			"branch-of",
			&point.to_string(),
		)])
		.await
		.unwrap();
		delete(&mut app, branch, ANN, photo.id())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("is inherited from the thread");
	}

	/// An edit replaces the body and logs the original, a deletion keeps the
	/// body under a tombstone, both in the store and the records alike, and a
	/// post edited twice has two logs naming it.
	#[beet_core::test]
	async fn the_records_keep_what_was_there() {
		let (mut app, root) = app();
		let window = window(&mut app, root, &[]).await.unwrap();
		let typo = say(&mut app, window, ANN, "helo");
		let gone = say(&mut app, window, ANN, "regret");
		edit(&mut app, window, ANN, typo.id(), "hello")
			.await
			.unwrap();
		edit(&mut app, window, ANN, typo.id(), "hello!")
			.await
			.unwrap();
		delete(&mut app, window, ANN, gone.id()).await.unwrap();
		let (_, stored) = stored(&mut app, window).await;
		for posts in [
			stored.clone(),
			records(&app, window)
				.posts()
				.iter()
				.map(|post| (**post).clone())
				.collect(),
		] {
			let post = |id| posts.iter().find(|post| post.id() == id).unwrap();
			post(typo.id()).text().xpect_eq(Some("hello!"));
			post(gone.id()).status().xpect_eq(PostStatus::Deleted);
			post(gone.id()).text().xpect_eq(Some("regret"));
			let logs = |subject: PostId| {
				posts
					.iter()
					.filter(|post| post.subject() == Some(subject))
					.collect::<Vec<_>>()
			};
			logs(typo.id())
				.iter()
				.map(|log| {
					log.log::<PostEdited>().unwrap().was["text"]
						.as_str()
						.unwrap()
						.to_string()
				})
				.collect::<Vec<_>>()
				.xpect_eq(vec!["helo".to_string(), "hello".to_string()]);
			logs(gone.id())[0].log::<PostDeleted>().xpect_some();
		}
		let records = records(&app, window);
		records.was_edited(typo.id()).xpect_true();
		records.was_edited(gone.id()).xpect_false();
	}

	/// A change is one batch: a log that cannot land leaves the post as it
	/// was.
	#[beet_core::test]
	async fn a_change_is_one_atomic_batch() {
		let (mut app, root) = app();
		let window = window(&mut app, root, &[]).await.unwrap();
		let first = say(&mut app, window, ANN, "first");
		let second = say(&mut app, window, ANN, "second");
		let (store, _) = stored(&mut app, window).await;
		let mut changed = first.clone();
		changed.set_text("changed");
		// a log at a key already taken, so its `Absent` fails the batch
		let mut log = Post::from_row(
			&second.key(),
			Post::new_log(ANN, first.thread(), &PostDeleted {})
				.unwrap()
				.into_row()
				.unwrap(),
		)
		.unwrap();
		log.set_thread(first.thread());
		store.change(&first, &[changed, log]).await.unwrap_err();
		let (_, stored) = stored(&mut app, window).await;
		stored
			.iter()
			.find(|post| post.id() == first.id())
			.unwrap()
			.text()
			.xpect_eq(Some("first"));
		stored
			.iter()
			.find(|post| post.id() == second.id())
			.unwrap()
			.text()
			.xpect_eq(Some("second"));
	}

	/// A change landing in the store after the window read the post refuses
	/// the window's own, never writing over it.
	#[beet_core::test]
	async fn a_conflicting_change_is_refused() {
		let (mut app, root) = app();
		let window = window(&mut app, root, &[]).await.unwrap();
		let said = say(&mut app, window, ANN, "mine");
		let (store, _) = stored(&mut app, window).await;
		// another runtime's edit, the window none the wiser
		let mut theirs = said.clone();
		theirs.set_text("theirs");
		let log = Post::new_log(ANN, said.thread(), &PostEdited {
			was: said.body().clone(),
		})
		.unwrap()
		.with_subject(said.id());
		store.change(&said, &[theirs, log]).await.unwrap();
		edit(&mut app, window, ANN, said.id(), "ours")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("changed since it was read");
		let (_, stored) = stored(&mut app, window).await;
		stored
			.iter()
			.find(|post| post.id() == said.id())
			.unwrap()
			.text()
			.xpect_eq(Some("theirs"));
	}

	/// A change reaches every window of the thread, written once by the
	/// window making it, and a window with no store changes its records the
	/// same way.
	#[beet_core::test]
	async fn a_change_reaches_every_window_once() {
		let (mut app, root) = app();
		let first = window(&mut app, root, &[]).await.unwrap();
		let second = window(&mut app, root, &[]).await.unwrap();
		let said = say(&mut app, first, ANN, "helo");
		app.update();
		edit(&mut app, second, ANN, said.id(), "hello")
			.await
			.unwrap();
		app.update();
		for window in [first, second] {
			let records = records(&app, window);
			records
				.post(said.id())
				.unwrap()
				.text()
				.xpect_eq(Some("hello"));
			records.was_edited(said.id()).xpect_true();
		}
		// neither flush writes the change again, which would conflict
		let (_, posts) = stored(&mut app, first).await;
		stored(&mut app, second).await;
		posts
			.iter()
			.filter(|post| post.kind() == &PostKind::Edited)
			.count()
			.xpect_eq(1);

		// with no store
		let storeless = app
			.world_mut()
			.spawn((Thread::default(), children![
				Actor::new("Ann", ActorKind::User).unwrap()
			]))
			.id();
		app.update();
		let said = say(&mut app, storeless, ANN, "helo");
		edit(&mut app, storeless, ANN, said.id(), "hello")
			.await
			.unwrap();
		records(&app, storeless)
			.post(said.id())
			.unwrap()
			.text()
			.xpect_eq(Some("hello"));
	}

	/// What is shown shares every post nobody touched with the records and
	/// copies only what it empties: a deleted post, read as deleted and
	/// nothing else, and an edit's original.
	#[beet_core::test]
	async fn shown_shares_what_nobody_touched() {
		let (mut app, root) = app();
		let window = window(&mut app, root, &[]).await.unwrap();
		let kept = say(&mut app, window, ANN, "kept");
		let typo = say(&mut app, window, ANN, "helo");
		let gone = say(&mut app, window, ANN, "gone");
		edit(&mut app, window, ANN, typo.id(), "hello")
			.await
			.unwrap();
		delete(&mut app, window, ANN, gone.id()).await.unwrap();
		let records = records(&app, window);
		let shown = records.shown(..);
		let pair = |id: PostId| {
			let index = records
				.posts()
				.iter()
				.position(|post| post.id() == id)
				.unwrap();
			(&records.posts()[index], &shown[index])
		};
		let (stored, read) = pair(kept.id());
		Arc::ptr_eq(stored, read).xpect_true();
		let (stored, read) = pair(gone.id());
		Arc::ptr_eq(stored, read).xpect_false();
		read.body().is_empty().xpect_true();
		matches!(read.as_agent_post(), Some(AgentPost::Deleted(_)))
			.xpect_true();
		for log in shown.iter().filter(|post| post.kind() == &PostKind::Edited)
		{
			log.body().is_empty().xpect_true();
		}
		// the edited post itself is shown as it now stands
		shown
			.post(typo.id())
			.unwrap()
			.text()
			.xpect_eq(Some("hello"));
		// a span holds only its posts
		records
			.shown(kept.id()..=kept.id())
			.iter()
			.map(|post| post.id())
			.collect::<Vec<_>>()
			.xpect_eq(vec![kept.id()]);
	}

	/// A window of Ann and a mock coach under `root` mounting `threads`,
	/// entered, and the coach's behavior entity.
	#[cfg(feature = "agent")]
	async fn conversation(app: &mut App, root: Entity) -> (Entity, Entity) {
		let window = app
			.world_mut()
			.spawn((
				ChildOf(root),
				Thread::default(),
				MountThreadStore::new("threads"),
				children![
					Actor::new("Ann", ActorKind::User).unwrap(),
					(
						Actor::new("Coach", ActorKind::Agent).unwrap(),
						MockPostStreamer::default()
					),
				],
			))
			.id();
		crate::streaming::request_test::enter(app, window).await;
		let coach = crate::streaming::request_test::actor_entity(app, &COACH);
		(window, coach)
	}

	#[cfg(feature = "agent")]
	const COACH: ActorId = ActorId::from_static("coach");

	/// Push `post` to `window`'s records, answering it.
	fn push(app: &mut App, window: Entity, post: Post) -> Post {
		app.world_mut()
			.get_mut::<ThreadRecords>(window)
			.unwrap()
			.upsert_post(post.clone());
		app.update();
		post
	}

	/// One post of every kind but the logs, by Ann and the coach.
	#[cfg(feature = "agent")]
	fn every_kind(thread: ThreadId) -> Vec<Post> {
		let done = PostStatus::Completed;
		let call = AgentPost::new_function_call(
			COACH, thread, "search", "c1", "{}", done,
		);
		vec![
			AgentPost::new_url(
				ANN,
				thread,
				"https://example.com/cat.png",
				MediaType::Png,
				None,
				done,
			),
			AgentPost::new_media(
				ANN,
				thread,
				MediaType::Png,
				b"cat".to_vec(),
				None,
				done,
			),
			AgentPost::new_refusal(COACH, thread, "no", done),
			AgentPost::new_error(COACH, thread, "failed", done),
			AgentPost::new_reasoning(COACH, thread, "hmm", done),
			AgentPost::new_reasoning_summary(COACH, thread, "thought", done),
			call.clone(),
			AgentPost::new_function_call_output(
				COACH,
				thread,
				call.id(),
				"c1",
				"found",
				done,
			),
		]
	}

	/// What Ann took back: an edit's original text, a deleted post's text and
	/// a deleted file's bytes, base64 encoded as a provider is sent them.
	#[cfg(feature = "agent")]
	const TAKEN_BACK: [&str; 3] =
		["ORIGINAL-SECRET", "DELETED-SECRET", "Z2hvc3Q="];

	/// Over a thread with every kind of post, the records and the store
	/// hold what was edited away and deleted after a reload, and neither
	/// what is shown, the model input of either protocol nor a route's
	/// answer holds any of it.
	#[cfg(feature = "agent")]
	#[beet_core::test]
	async fn nothing_shown_holds_what_was_taken_back() {
		let (mut app, root) = app();
		app.world_mut().entity_mut(root).insert(BlobStore::temp());
		let (window, coach) = conversation(&mut app, root).await;
		let thread = app.world().get::<Thread>(window).unwrap().id();
		for post in every_kind(thread) {
			push(&mut app, window, post);
		}
		let done = PostStatus::Completed;
		let edited = say(&mut app, window, ANN, "ORIGINAL-SECRET");
		let deleted = say(&mut app, window, ANN, "DELETED-SECRET");
		let photo = push(
			&mut app,
			window,
			AgentPost::new_media(
				ANN,
				thread,
				MediaType::Png,
				b"ghost".to_vec(),
				None,
				done,
			),
		);
		edit(&mut app, window, ANN, edited.id(), "public")
			.await
			.unwrap();
		delete(&mut app, window, ANN, deleted.id()).await.unwrap();
		delete(&mut app, window, ANN, photo.id()).await.unwrap();
		// the coach answers with what it reads, as a route's answer
		push(
			&mut app,
			window,
			AgentPost::new_text(COACH, thread, "noted", done),
		);
		let holds_any = |text: &str| {
			TAKEN_BACK[..2].iter().any(|secret| text.contains(secret))
		};

		// everything, reloaded from the store
		let (store, posts) = stored(&mut app, window).await;
		holds_any(&format!("{posts:?}")).xpect_true();
		let reloaded = store
			.open(&store.thread(thread).await.unwrap().unwrap(), Reach::Whole)
			.await
			.unwrap();
		for secret in &TAKEN_BACK[..2] {
			format!("{:?}", reloaded.posts()).xpect_contains(*secret);
		}
		store
			.read_blob(photo.files()[0].blob())
			.await
			.unwrap()
			.to_vec()
			.xpect_eq(b"ghost".to_vec());

		// shown, by the window and by records reloaded
		for records in [records(&app, window), &reloaded] {
			holds_any(&format!("{:?}", records.shown(..))).xpect_false();
			records
				.shown(..)
				.post(edited.id())
				.unwrap()
				.text()
				.xpect_eq(Some("public"));
		}
		// what either protocol sends, the file's bytes included
		let requests =
			crate::streaming::request_test::requests(&mut app, coach)
				.await
				.map(|request| request.to_json_string());
		for request in &requests {
			for secret in TAKEN_BACK {
				request.as_str().xnot().xpect_contains(secret);
			}
			request.as_str().xpect_contains("public");
			request.as_str().xpect_contains("Ann edited a post from ");
			request.as_str().xpect_contains("Ann deleted a post from ");
			// the file nobody took back is carried
			request.as_str().xpect_contains("Y2F0");
		}
	}

	/// A request never chains to a response whose read covers a post changed
	/// since: it chains to the newest that does not, else sends everything,
	/// the change read as a note where it was made.
	#[cfg(feature = "agent")]
	#[beet_core::test]
	async fn a_request_never_chains_past_a_change() {
		let (mut app, root) = app();
		let (window, coach) = conversation(&mut app, root).await;
		let thread = app.world().get::<Thread>(window).unwrap().id();
		// the coach answers once before Ann's typo and once after it
		let answer = |app: &mut App, response: &str| {
			let post = AgentPost::new_text(
				COACH,
				thread,
				"hello",
				PostStatus::Completed,
			);
			let mut records =
				app.world_mut().get_mut::<ThreadRecords>(window).unwrap();
			let read = records.read_through();
			records.set_meta(ResponseMeta {
				post_id: post.id(),
				provider_slug: "provider".into(),
				model_slug: "model".into(),
				response_id: response.into(),
				response_stored: true,
				usage: None,
				read,
			});
			records.upsert_post(post);
		};
		let early = say(&mut app, window, ANN, "hi");
		answer(&mut app, "response-1");
		let typo = say(&mut app, window, ANN, "helo");
		answer(&mut app, "response-2");
		let request = async |app: &mut App| {
			crate::streaming::request_test::responses_request(
				app,
				coach,
				O11sStreamer::new(crate::streaming::request_test::model())
					.with_use_previous_response_id(),
			)
			.await
		};
		request(&mut app)
			.await
			.previous_response_id
			.as_deref()
			.xpect_eq(Some("response-2"));
		edit(&mut app, window, ANN, typo.id(), "hello")
			.await
			.unwrap();
		let chained = request(&mut app).await;
		chained
			.previous_response_id
			.as_deref()
			.xpect_eq(Some("response-1"));
		let input = Value::from_serde(&chained.input).unwrap().to_json_string();
		input.as_str().xnot().xpect_contains("helo<");
		input.as_str().xpect_contains("hello</post>");
		input.as_str().xpect_contains("Ann edited a post from ");

		// a change under every response's read leaves nothing to chain to
		delete(&mut app, window, ANN, early.id()).await.unwrap();
		let unchained = request(&mut app).await;
		unchained.previous_response_id.xpect_none();
		let input = Value::from_serde(&unchained.input)
			.unwrap()
			.to_json_string();
		input.as_str().xnot().xpect_contains(">hi<");
		input.as_str().xpect_contains("Ann deleted a post from ");
	}
}
