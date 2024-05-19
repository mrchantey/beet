use crate::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;

pub fn handle_spawn_outgoing(
	mut outgoing: EventWriter<MessageOutgoing>,
	query: Query<Entity, Added<Replicate>>,
) {
	for entity in query.iter() {
		outgoing.send(Message::Spawn { entity }.into());
	}
}

pub fn handle_despawn_outgoing(
	mut outgoing: EventWriter<MessageOutgoing>,
	mut removed: RemovedComponents<Replicate>,
) {
	for entity in removed.read() {
		outgoing.send(Message::Despawn { entity }.into());
	}
}
pub fn handle_incoming(
	mut commands: Commands,
	mut registrations: ResMut<Registrations>,
	mut incoming: EventReader<MessageIncoming>,
) {
	for msg in incoming.read() {
		match &**msg {
			Message::Spawn { entity } => {
				let local = commands.spawn_empty().id();
				registrations.entities.insert(*entity, local);
			}
			Message::Despawn { entity } => {
				commands.entity(*entity).despawn();
			}
			Message::Insert {
				entity,
				reg_id,
				bytes,
			} => {
				if let Some((entity, fns)) =
					registrations.entity_fns(*entity, *reg_id)
				{
					(fns.insert)(&mut commands.entity(entity), bytes)
						.ok_or(|e| log::error!("{e}"));
				} else {
				}
			}
			Message::Change {
				entity,
				reg_id,
				bytes,
			} => {
				if let Some((entity, fns)) =
					registrations.entity_fns(*entity, *reg_id)
				{
					(fns.change)(&mut commands.entity(entity), bytes)
						.ok_or(|e| log::error!("{e}"));
				} else {
				}
			}
			Message::Remove { entity, reg_id } => {
				if let Some((entity, fns)) =
					registrations.entity_fns(*entity, *reg_id)
				{
					(fns.remove)(&mut commands.entity(entity));
				} else {
				}
			}
		}
	}
}
