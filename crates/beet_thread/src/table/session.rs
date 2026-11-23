//! Sessions as posts, the words defined in the [`table`](crate::table)
//! module docs.
//!
//! Entering a window appends a [`SessionOpened`] post, a join, for each
//! actor its scene authors that no other window of the runtime holds and that
//! is not of kind System, authored by that actor. The end of a window's life
//! appends a [`SessionClosed`] post authored by the thread's System actor.
//! Model input skips both ([`ThreadRecords::model_posts`]) and a `ThreadView`
//! renders them as dividers.
use crate::prelude::*;
#[cfg(feature = "action")]
use beet_action::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "action")]
use std::time::Duration;

/// The log of an actor joining its thread, authored by the actor.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOpened {
	/// The post whose action asked for the entry, ie the loop decision
	/// requesting the unit. A thread keeps its creator in
	/// [`Thread::spawned_by`]; each join records its entry's cause here.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub spawned_by: Option<PostRef>,
}

/// The log of a session leaving its thread.
#[cfg(feature = "action")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionClosed {
	/// How the session's behavior ended, an error closing it as a fail.
	pub outcome: Outcome,
	/// From the session's opened post to this one.
	pub duration: Duration,
	/// Every response the session's agents received, totalled.
	pub usage: Usage,
}

impl LogKind for SessionOpened {
	const KIND: &'static str = "session_opened";
}

impl SessionOpened {
	/// Whether `post` opens a session.
	pub fn opens(post: &Post) -> bool {
		post.log_kind().as_deref() == Some(Self::KIND)
	}
}

#[cfg(feature = "action")]
impl LogKind for SessionClosed {
	const KIND: &'static str = "session_closed";
}

impl ThreadRecords {
	/// The thread's System actor, the author of its [`SessionClosed`] posts:
	/// the first by id the records hold, or one created here if the template
	/// authored none.
	#[cfg(feature = "action")]
	pub fn system_actor(&mut self) -> ActorId {
		self.actors()
			.values()
			.filter(|actor| actor.kind() == ActorKind::System)
			.map(Actor::id)
			.min()
			.unwrap_or_else(|| self.insert_actor(Actor::system()))
	}

	/// Append a [`SessionOpened`] post for each of `actors` that is not of
	/// kind System, authored by it.
	///
	/// ## Errors
	///
	/// Errors when an actor is not in the records.
	pub fn join(
		&mut self,
		thread: ThreadId,
		actors: impl IntoIterator<Item = ActorId>,
		opened: &SessionOpened,
	) -> Result {
		for actor in actors {
			if self.actor(actor)?.kind() == ActorKind::System {
				continue;
			}
			self.upsert_post(Post::new_log(actor, thread, opened)?);
		}
		Ok(())
	}

	/// Append the [`SessionClosed`] post ending the session the last
	/// [`SessionOpened`] post opened, totalling the usage of every response
	/// since.
	///
	/// ## Errors
	///
	/// Errors when no session is open.
	#[cfg(feature = "action")]
	pub fn close_session(
		&mut self,
		thread: ThreadId,
		outcome: Outcome,
	) -> Result<SessionClosed> {
		let opened = self
			.posts()
			.iter()
			.rev()
			.find(|post| SessionOpened::opens(post))
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
		let post = Post::new_log(self.system_actor(), thread, &closed)?;
		self.upsert_post(post);
		closed.xok()
	}
}

#[cfg(feature = "action")]
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
				match world.get_mut::<ThreadRecords>(entity) {
					Some(mut records) => {
						records.close_session(id, outcome).map(|_| ())
					}
					None => Ok(()),
				}
			})
			.await
	}
}
