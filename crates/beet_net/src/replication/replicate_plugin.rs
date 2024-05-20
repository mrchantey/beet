use crate::prelude::*;
use bevy::prelude::*;




#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which messages are inserted.
pub struct PreMessageSet;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which messages are retrieved and sent.
pub struct MessageSet;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which messages are sent.
pub struct PostMessageSet;


#[derive(Default, Component)]
pub struct Replicate {}

pub struct ReplicatePlugin;

impl Plugin for ReplicatePlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.init_resource::<Registrations>()
			.init_resource::<MessageIncoming>()
			.init_resource::<MessageOutgoing>()
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
