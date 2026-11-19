//! Sessions as posts: the bracket around one thread entity's life.
//!
//! A session is one thread entity's life, from entry to despawn: one
//! [`ThreadRoute`] request, one [`RunThread`] entry. Entry appends a
//! [`SessionOpened`] post and the end a [`SessionClosed`] post, both
//! informational ([`PostIntent::SESSION_OPENED`],
//! [`PostIntent::SESSION_CLOSED`]) with a typed JSON body, authored by the
//! thread's System actor. Model input skips them ([`ThreadWindow::model_posts`])
//! and a `ThreadView` renders them as dividers. An opened post with no closed
//! post before the next opened post, or before the end, is a session that
//! aborted.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::time::Duration;

/// The body of a [`PostIntent::SESSION_OPENED`] post.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOpened {
	/// The post whose action asked for this session, ie the loop decision
	/// requesting the unit. A thread keeps its creator in
	/// [`Thread::spawned_by`]; each session records its own cause here.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub spawned_by: Option<PostRef>,
}

/// The body of a [`PostIntent::SESSION_CLOSED`] post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionClosed {
	/// How the session's behavior ended, an error closing it as a fail.
	pub outcome: Outcome,
	/// From the session's opened post to this one.
	pub duration: Duration,
	/// Every response the session's agents received, totalled.
	pub usage: Usage,
}

impl SessionOpened {
	/// The body of `post`, if it opens a session.
	pub fn of(post: &Post) -> Option<Self> {
		session_body(post, PostIntent::SESSION_OPENED)
	}
}

impl SessionClosed {
	/// The body of `post`, if it closes a session.
	pub fn of(post: &Post) -> Option<Self> {
		session_body(post, PostIntent::SESSION_CLOSED)
	}
}

/// The JSON body of `post` when it carries `intent`.
fn session_body<T: serde::de::DeserializeOwned>(
	post: &Post,
	intent: PostIntent,
) -> Option<T> {
	(post.intent() == intent)
		.then(|| serde_json::from_slice(post.body_bytes()).ok())
		.flatten()
}

impl ThreadWindow {
	/// The thread's System actor, the author of its session posts: the first
	/// by id the window holds, or one created here if the template authored
	/// none, so every template brackets its sessions unchanged.
	pub fn system_actor(&mut self) -> ActorId {
		self.actors()
			.values()
			.filter(|actor| actor.kind() == ActorKind::System)
			.map(Actor::id)
			.min()
			.unwrap_or_else(|| self.insert_actor(Actor::system()))
	}

	/// Append the [`SessionOpened`] post of a session entering `thread`.
	pub fn open_session(
		&mut self,
		thread: ThreadId,
		opened: SessionOpened,
	) -> Result<PostId> {
		let post = self.session_post(
			thread,
			PostIntent::SESSION_OPENED,
			serde_json::to_vec(&opened)?,
		);
		let id = post.id();
		self.upsert_post(post);
		id.xok()
	}

	/// Append the [`SessionClosed`] post ending the session the last
	/// [`SessionOpened`] post opened, totalling the usage of every response
	/// since.
	///
	/// ## Errors
	///
	/// Errors when no session is open.
	pub fn close_session(
		&mut self,
		thread: ThreadId,
		outcome: Outcome,
	) -> Result<SessionClosed> {
		let opened = self
			.posts()
			.iter()
			.rev()
			.find(|post| post.intent() == PostIntent::SESSION_OPENED)
			.ok_or_else(|| bevyhow!("thread {thread} has no open session"))?;
		let opened_id = opened.id();
		let duration = Timestamp::now().millis() - opened.created().millis();
		let closed = SessionClosed {
			outcome,
			duration: Duration::from_millis(duration.max(0) as u64),
			usage: self
				.posts()
				.iter()
				.filter(|post| post.id() > opened_id)
				.filter_map(|post| self.meta(post.id())?.usage)
				.sum(),
		};
		let post = self.session_post(
			thread,
			PostIntent::SESSION_CLOSED,
			serde_json::to_vec(&closed)?,
		);
		self.upsert_post(post);
		closed.xok()
	}

	/// A completed session post by the System actor.
	fn session_post(
		&mut self,
		thread: ThreadId,
		intent: PostIntent,
		body: Vec<u8>,
	) -> Post {
		Post::new_raw(
			self.system_actor(),
			thread,
			intent,
			MediaType::Json,
			body,
			default(),
		)
	}
}

impl Thread {
	/// Close the session of the thread at `entity` with `outcome`, appending
	/// its [`SessionClosed`] post. A thread already gone has nothing to close.
	///
	/// ## Errors
	///
	/// Errors when the thread opened no session.
	pub async fn close(
		world: &AsyncWorld,
		entity: Entity,
		outcome: Outcome,
	) -> Result {
		world
			.with(move |world: &mut World| -> Result {
				let Some(id) = world.get::<Thread>(entity).map(Thread::id)
				else {
					return Ok(());
				};
				match world.get_mut::<ThreadWindow>(entity) {
					Some(mut window) => {
						window.close_session(id, outcome).map(|_| ())
					}
					None => Ok(()),
				}
			})
			.await
	}
}
