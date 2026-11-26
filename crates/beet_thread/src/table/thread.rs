use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A conversation's record, for its whole life, named by its [`ThreadId`]: the
/// one row of its partition's `thread` table, and a component on each window
/// open onto it. Its id carries the time it was created.
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
	/// The row's key, never in its body.
	#[serde(skip)]
	id: ThreadId,
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

impl TableStoreRow for Thread {
	fn table_name() -> SmolStr { "thread".into() }
	fn key(&self) -> TableKey { self.id.into() }
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		let mut thread = row.into_serde::<Self>()?;
		thread.id = ThreadId::parse(key)?;
		thread.xok()
	}
}

impl Thread {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			id: ThreadId::now(),
			name: name.into(),
			spawned_by: None,
			branch_of: None,
		}
	}

	pub fn id(&self) -> ThreadId { self.id }
	/// The instant the thread was created, its id's.
	pub fn created(&self) -> Timestamp { self.id.created() }
	/// Mint this record a new id, for a thread whose id another already took.
	#[cfg(any(test, feature = "action"))]
	pub(crate) fn remint(&mut self) { self.id = ThreadId::now(); }
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
