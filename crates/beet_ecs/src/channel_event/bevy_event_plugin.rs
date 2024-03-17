use super::*;
use bevy::prelude::*;



#[derive(Clone)]
pub struct BevyEventPlugin {
	pub world_handler: WorldHandler,
	pub component_change_send: ComponentChangeSend,
	pub component_change_recv: ComponentChangeRecv,
}

impl BevyEventPlugin {
	pub fn new(registry: AppTypeRegistry) -> Self {
		Self {
			world_handler: WorldHandler::new(),
			component_change_send: ComponentChangeSend::new(registry.clone()),
			component_change_recv: ComponentChangeRecv::new(registry),
		}
	}
}

impl Plugin for BevyEventPlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.insert_resource(self.world_handler.clone())
			.add_systems(PreUpdate, WorldHandler::system)
			.insert_resource(self.component_change_recv.clone())
			.add_systems(PreUpdate, ComponentChangeRecv::system)
			.insert_resource(self.component_change_send.clone())
			.add_systems(PostUpdate, ComponentChangeSend::system)
			/*-*/;
	}
}
