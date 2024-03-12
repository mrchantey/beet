use crate::prelude::*;
use bevy_math::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct SpawnEntityPayload<T: ActionTypes> {
	pub beet_id: BeetEntityId,
	pub name: String,
	pub position: Option<Vec3>,
	// #[serde(deserialize_with = "BehaviorPrefab::<T>::deserialize")]
	pub prefab: Option<BehaviorPrefab<T>>,
	pub position_tracking: bool,
}

// impl<T: ActionTypes> Default for SpawnEntityPayload<T> {
// 	fn default() -> Self {
// 		Self {
// 			name: "New Entity".to_string(),
// 			position: None,
// 			prefab: None,
// 			position_tracking: false,
// 		}
// 	}
// }

impl<T: ActionTypes> SpawnEntityPayload<T> {
	pub fn from_id(beet_id: impl Into<BeetEntityId>) -> Self {
		Self {
			beet_id: beet_id.into(),
			name: "New Entity".to_string(),
			position: None,
			prefab: None,
			position_tracking: false,
		}
	}
	pub fn new(
		beet_id: BeetEntityId,
		name: String,
		graph: Option<BehaviorPrefab<T>>,
		position: Option<Vec3>,
		position_tracking: bool,
	) -> Self {
		Self {
			beet_id,
			name,
			position,
			prefab: graph,
			position_tracking,
		}
	}
	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		self.name = name.into();
		self
	}

	pub fn with_position(mut self, position: Vec3) -> Self {
		self.position = Some(position);
		self
	}
	pub fn with_tracked_position(mut self, position: Vec3) -> Self {
		self.position_tracking = true;
		self.position = Some(position);
		self
	}
	pub fn with_prefab(mut self, graph: impl Into<BehaviorPrefab<T>>) -> Self {
		self.prefab = Some(graph.into());
		self
	}
}
