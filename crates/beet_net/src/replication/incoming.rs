use crate::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;


pub fn handle_incoming_commands(
	mut commands: Commands,
	mut registrations: ResMut<ReplicateRegistry>,
	incoming: Res<MessageIncoming>,
) {
	for msg in incoming.iter() {
		match msg {
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
				payload,
			} => {
				if let Some((entity, fns)) =
					registrations.entity_fns(*entity, *reg_id)
				{
					(fns.insert)(&mut commands.entity(entity), &payload)
						.ok_or(|e| log::error!("{e}"));
				}
			}
			Message::Change {
				entity,
				reg_id,
				payload,
			} => {
				if let Some((entity, fns)) =
					registrations.entity_fns(*entity, *reg_id)
				{
					(fns.change)(&mut commands.entity(entity), payload)
						.ok_or(|e| log::error!("{e}"));
				}
			}
			Message::Remove { entity, reg_id } => {
				if let Some((entity, fns)) =
					registrations.entity_fns(*entity, *reg_id)
				{
					(fns.remove)(&mut commands.entity(entity));
				}
			}
			Message::InsertResource { reg_id, payload } => {
				if let Some(fns) = registrations.resources.get(reg_id) {
					(fns.insert)(&mut commands, payload)
						.ok_or(|e| log::error!("{e}"));
				}
			}
			Message::ChangeResource { reg_id, payload } => {
				if let Some(fns) = registrations.resources.get(reg_id) {
					(fns.change)(&mut commands, payload)
						.ok_or(|e| log::error!("{e}"));
				}
			}
			Message::RemoveResource { reg_id } => {
				if let Some(fns) = registrations.resources.get(reg_id) {
					(fns.remove)(&mut commands);
				}
			}
			Message::SendEvent {
				reg_id: _,
				payload: _,
			} => {
				// events require world access
			}
		}
	}
}

pub fn handle_incoming_world(world: &mut World) {
	let registrations = world.resource::<ReplicateRegistry>();
	let events = world
		.resource::<MessageIncoming>()
		.iter()
		.filter_map(|msg| match msg {
			Message::SendEvent { reg_id, payload } => {
				if let Some(fns) = registrations.events.get(reg_id) {
					Some((fns.clone(), payload.clone()))
				} else {
					None
				}
			}
			_ => None,
		})
		.collect::<Vec<_>>();

	for (fns, payload) in events {
		(fns.send)(world, &payload).ok_or(|e| log::error!("{e}"));
	}
}
