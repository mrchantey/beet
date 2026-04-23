use crate::prelude::*;
use beet_core::prelude::*;

pub type ThreadId = Uuid7<Thread>;

#[derive(
	Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, Component,
)]
#[reflect(Serialize, Deserialize, Component)]
pub struct Thread {
	id: ThreadId,
	created: Timestamp,
	name: String,
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
			metadata: default(),
		}
	}

	pub fn created(&self) -> Timestamp { self.created }
	pub fn name(&self) -> &str { &self.name }

	pub fn metadata(&self) -> &Map { &self.metadata }
	pub fn metadata_mut(&mut self) -> &mut Map { &mut self.metadata }
}

impl std::hash::Hash for Thread {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id.hash(state);
		self.created.hash(state);
		self.name.hash(state);
		// metadata excluded: HashMap does not implement Hash
	}
}
