use crate::prelude::*;
use bevy::prelude::*;




#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which messages parsed.
pub struct MessageSet;


#[derive(Default, Component)]
pub struct Replicate {}

pub struct ReplicatePlugin;

impl Plugin for ReplicatePlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.init_resource::<Registrations>()
			.add_event::<MessageIncoming>()
			.add_event::<MessageOutgoing>()
			.add_systems(
				Update,
				(
					handle_spawn_outgoing,
					handle_despawn_outgoing,
					handle_incoming,
				)
					.chain()
					.in_set(MessageSet),
			);
	}
}
