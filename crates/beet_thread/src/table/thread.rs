use crate::prelude::*;
use beet_core::prelude::*;

pub type ThreadId = Uuid7<Thread>;

/// A conversation's record, for its whole life, named by its [`ThreadId`]: one
/// row in its store, and a component on each window open onto it.
///
/// Two relations name another thread's post, each by [`PostRef`] and neither
/// mirrored by an entity relationship, so the tree of threads reads from
/// records alone:
///
/// - [`Self::spawned_by`]: the post whose action started this thread, ie a
///   loop's decision, a delegate call or a person opening a sub-thread. Causal
///   only, no context comes with it.
/// - [`Self::branch_of`]: the post this thread continues the history of before
///   going its own way. Its history is its ancestors' posts up to each branch
///   point, referenced rather than copied, then its own.
///
/// A thread may be both: a sub-thread opening with its parent's history is
/// spawned by and branched from the same post.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
	Component,
)]
#[reflect(Default, Serialize, Deserialize, Component)]
pub struct Thread {
	id: ThreadId,
	created: Timestamp,
	name: String,
	/// The post whose action started this thread.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	spawned_by: Option<PostRef>,
	/// The post this thread branches from, continuing its history.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	branch_of: Option<PostRef>,
}

impl Default for Thread {
	fn default() -> Self { Self::new("New Thread") }
}

impl ThreadRecord for Thread {
	type Id = ThreadId;
	fn id(&self) -> Self::Id { self.id }
}

impl Thread {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			id: Uuid7::new_now(),
			created: Timestamp::now(),
			name: name.into(),
			spawned_by: None,
			branch_of: None,
		}
	}

	pub fn created(&self) -> Timestamp { self.created }
	pub fn name(&self) -> &str { &self.name }

	/// The post whose action started this thread.
	pub fn spawned_by(&self) -> Option<&PostRef> { self.spawned_by.as_ref() }
	pub fn set_spawned_by(&mut self, spawned_by: Option<PostRef>) {
		self.spawned_by = spawned_by;
	}
	pub fn with_spawned_by(mut self, spawned_by: PostRef) -> Self {
		self.spawned_by = Some(spawned_by);
		self
	}

	/// The post this thread branches from.
	pub fn branch_of(&self) -> Option<&PostRef> { self.branch_of.as_ref() }
	pub fn set_branch_of(&mut self, branch_of: Option<PostRef>) {
		self.branch_of = branch_of;
	}
	pub fn with_branch_of(mut self, branch_of: PostRef) -> Self {
		self.branch_of = Some(branch_of);
		self
	}
}
