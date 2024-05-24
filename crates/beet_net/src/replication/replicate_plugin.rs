use crate::prelude::*;
use bevy::prelude::*;




#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which [`MessageIncoming`] messages are read.
pub struct MessageIncomingSet;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which [`MessageOutgoing`] messages are written.
pub struct MessageOutgoingSet;


#[derive(Default, Component)]
pub struct Replicate {}



/**
A typical replication setup would look something like this:
- [`transport_incoming`]: [`MessageIncoming`] is appended by the transport
- [`MessageIncomingSet`]: [`MessageIncoming`] is read by registered systems
- [`MessageOutgoingSet`]: [`MessageOutgoing`] is appended by registered systems
- [`clear_incoming`]: [`MessageIncoming`] is cleared
- [`transport_outgoing`]: [`MessageOutgoing`] is cleared and sent by the transport
**/
pub struct ReplicatePlugin;

impl Plugin for ReplicatePlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
			.add_plugins(ReplicateEntityPlugin)
			.configure_sets(
				Update,
				MessageIncomingSet.before(MessageOutgoingSet),
			)
			.init_resource::<Registrations>()
			.init_resource::<MessageIncoming>()
			.init_resource::<MessageOutgoing>()
			.add_systems(Update, clear_incoming.after(MessageIncomingSet));

		#[cfg(feature = "beet_ecs")]
		{
			use beet_ecs::prelude::*;
			app.configure_sets(Update, MessageIncomingSet.in_set(PreTickSet));
			app.configure_sets(Update, MessageOutgoingSet.in_set(PostTickSet));
		}
	}
}


fn clear_incoming(mut incoming: ResMut<MessageIncoming>) { incoming.clear(); }
