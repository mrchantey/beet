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
			Message::Add {
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
				if let Some(fns) =
					registrations.incoming_resource_fns.get(reg_id)
				{
					(fns.insert)(&mut commands, payload)
						.ok_or(|e| log::error!("{e}"));
				}
			}
			Message::ChangeResource { reg_id, payload } => {
				if let Some(fns) =
					registrations.incoming_resource_fns.get(reg_id)
				{
					(fns.change)(&mut commands, payload)
						.ok_or(|e| log::error!("{e}"));
				}
			}
			Message::RemoveResource { reg_id } => {
				if let Some(fns) =
					registrations.incoming_resource_fns.get(reg_id)
				{
					(fns.remove)(&mut commands);
				}
			}
			Message::SendObserver { reg_id, payload } => {
				if let Some(fns) =
					registrations.incoming_observer_fns.get(reg_id)
				{
					(fns.send)(&mut commands, payload)
						.ok_or(|e| log::error!("{e}"));
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
				if let Some(fns) = registrations.incoming_event_fns.get(reg_id)
				{
					Some((fns.clone(), payload.clone()))
				} else {
					None
				}
			}
			_ => {
				// all other messages are handled by `handle_incoming_commands`
				None
			}
		})
		.collect::<Vec<_>>();

	for (fns, payload) in events {
		(fns.send)(world, &payload).ok_or(|e| log::error!("{e}"));
	}
}
