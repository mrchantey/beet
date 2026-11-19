use crate::prelude::*;
use beet_core::prelude::*;

pub type ThreadId = Uuid7<Thread>;

/// A conversation, for its whole life: the record every session continuing it
/// adopts, named by its [`ThreadId`].
///
/// Two relations name another thread's post, each by [`PostRef`] and neither
/// mirrored by an entity relationship, so the navigator reads the tree from
/// records alone:
///
/// - [`Self::spawned_by`]: the post whose action started this thread, ie a
///   loop's decision or a delegate call. Causal only, no context comes with it.
/// - [`Self::branch_of`]: the post this thread continues the history of before
///   going its own way. Its history is its ancestors' posts up to each branch
///   point, referenced rather than copied, then its own.
///
/// A thread may be both: a subagent seeing its parent's context is spawned by
/// and branched from the same delegating post.
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
	/// Extensible key-value metadata, the entries a request continues a
	/// stored thread by ([`ThreadEntry`]).
	metadata: Map,
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
			metadata: default(),
		}
	}

	pub fn created(&self) -> Timestamp { self.created }
	pub fn name(&self) -> &str { &self.name }

	/// Adopt a stored [`ThreadId`] when an entry continues a stored thread, so
	/// posts created this session share its identity. The id is minted fresh
	/// on build and adopted on entry (actor ids, by contrast, are authored and
	/// stable).
	pub fn set_id(&mut self, id: ThreadId) { self.id = id; }

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

	pub fn metadata(&self) -> &Map { &self.metadata }
	pub fn metadata_mut(&mut self) -> &mut Map { &mut self.metadata }
	pub fn with_metadata(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Self {
		self.metadata.insert(key, value);
		self
	}

	/// Whether this record's metadata holds every entry of `entries`.
	pub fn matches(&self, entries: &Map) -> bool {
		entries
			.iter()
			.all(|(key, value)| self.metadata.get(key).ok() == Some(value))
	}
}
