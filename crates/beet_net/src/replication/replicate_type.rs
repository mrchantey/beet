use crate::prelude::*;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;


#[extend::ext(name=AppExtReplicate)]
pub impl App {
	fn replicate<T: Component + Serialize + DeserializeOwned>(
		&mut self,
	) -> &mut Self {
		self.replicate_with::<T>(ReplicateDirection::Both)
	}
	fn replicate_with<T: Component + Serialize + DeserializeOwned>(
		&mut self,
		direction: ReplicateDirection,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_component::<T>(direction);
		if direction.is_outgoing() {
			register_component_outgoing::<T>(self);
		}
		self
	}
	fn replicate_resource_incoming<
		T: Resource + Serialize + DeserializeOwned,
	>(
		&mut self,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_resource::<T>(ReplicateDirection::Incoming);
		self
	}
	fn replicate_resource_outgoing<
		T: Resource + Serialize + DeserializeOwned,
	>(
		&mut self,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_resource::<T>(ReplicateDirection::Incoming);
		register_resource_outgoing::<T>(self);
		self
	}

	fn replicate_event_incoming<T: Event + Serialize + DeserializeOwned>(
		&mut self,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_event::<T>(ReplicateDirection::Incoming);
		self
	}
	fn replicate_event_outgoing<T: Event + Serialize + DeserializeOwned>(
		&mut self,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_event::<T>(ReplicateDirection::Incoming);
		register_event_outgoing::<T>(self);
		self
	}
	fn replicate_observer_incoming<T: Event + Serialize + DeserializeOwned>(
		&mut self,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_observer::<T>(ReplicateDirection::Incoming);
		self
	}
	fn replicate_observer_outgoing<T: Event + Serialize + DeserializeOwned>(
		&mut self,
	) -> &mut Self {
		self.init_resource::<ReplicateRegistry>()
			.world_mut()
			.resource_mut::<ReplicateRegistry>()
			.register_observer::<T>(ReplicateDirection::Incoming);
		register_observer_outgoing::<T>(self);
		self
	}
}
