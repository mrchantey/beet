use super::*;
use bevy::prelude::*;



pub struct ChannelEventPlugin {
	command_handler: CommandHandler,
	component_change_send: ComponentChangeSend,
	component_change_recv: ComponentChangeRecv,
}


impl Plugin for ChannelEventPlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.insert_resource(self.command_handler.clone())
			.insert_resource(self.component_change_recv.clone())
			.add_systems(PreUpdate, ComponentChangeRecv::system)
			.add_systems(PreUpdate, CommandHandler::system)
			.insert_resource(self.component_change_send.clone())
			.add_systems(PostUpdate, ComponentChangeSend::system)
			/*-*/;
	}
}
