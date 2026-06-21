use crate::prelude::*;
use beet_core::prelude::*;

pub type ThreadId = Uuid7<Thread>;

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
	/// Hash of the author scene's seed (actor definitions + seed-post content
	/// and author, excluding volatile post ids). The stable thread identity:
	/// editing the seed forks a new thread, editing only behavior continues it.
	seed_hash: u64,
	/// Extensible key-value metadata.
	metadata: Map,
}

impl Default for Thread {
	fn default() -> Self { Self::new("New Thread") }
}

impl Table for Thread {
	type Id = ThreadId;
	fn id(&self) -> Self::Id { self.id }
}

impl Thread {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			id: Uuid7::new_now(),
			created: Timestamp::now(),
			name: name.into(),
			seed_hash: 0,
			metadata: default(),
		}
	}

	pub fn created(&self) -> Timestamp { self.created }
	pub fn name(&self) -> &str { &self.name }

	pub fn seed_hash(&self) -> u64 { self.seed_hash }
	pub fn set_seed_hash(&mut self, seed_hash: u64) {
		self.seed_hash = seed_hash;
	}

	/// Adopt a stored [`ThreadId`] when a seed-hash match loads an existing
	/// thread from its [`ThreadStore`], so posts created this session share the
	/// stored thread identity. The thread id is minted fresh on bootstrap and
	/// adopted on load (actor ids, by contrast, are authored and stable).
	pub fn set_id(&mut self, id: ThreadId) { self.id = id; }

	pub fn metadata(&self) -> &Map { &self.metadata }
	pub fn metadata_mut(&mut self) -> &mut Map { &mut self.metadata }
}
