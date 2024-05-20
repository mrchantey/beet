use crate::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;

pub fn handle_spawn_outgoing(
	mut outgoing: ResMut<MessageOutgoing>,
	query: Query<Entity, Added<Replicate>>,
) {
	for entity in query.iter() {
		outgoing.push(Message::Spawn { entity }.into());
	}
}

pub fn handle_despawn_outgoing(
	mut outgoing: ResMut<MessageOutgoing>,
	mut removed: RemovedComponents<Replicate>,
) {
	for entity in removed.read() {
		outgoing.push(Message::Despawn { entity }.into());
	}
}
pub fn handle_incoming(
	mut commands: Commands,
	mut registrations: ResMut<Registrations>,
	mut incoming: ResMut<MessageIncoming>,
) {
	for msg in incoming.drain(..) {
		match msg {
			Message::Spawn { entity } => {
				let local = commands.spawn_empty().id();
				registrations.entities.insert(entity, local);
			}
			Message::Despawn { entity } => {
				commands.entity(entity).despawn();
			}
			Message::Insert {
				entity,
				reg_id,
				bytes,
			} => {
				if let Some((entity, fns)) =
					registrations.entity_fns(entity, reg_id)
				{
					(fns.insert)(&mut commands.entity(entity), &bytes)
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
					registrations.entity_fns(entity, reg_id)
				{
					(fns.change)(&mut commands.entity(entity), &bytes)
						.ok_or(|e| log::error!("{e}"));
				} else {
				}
			}
			Message::Remove { entity, reg_id } => {
				if let Some((entity, fns)) =
					registrations.entity_fns(entity, reg_id)
				{
					(fns.remove)(&mut commands.entity(entity));
				} else {
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;



	#[test]
	fn outgoing() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ReplicatePlugin);

		let entity = app.world_mut().spawn(Replicate::default()).id();

		app.update();
		app.world_mut().despawn(entity);
		app.update();

		let events = app.world_mut().resource_mut::<MessageOutgoing>();
		expect(events.len()).to_be(2)?;
		expect(&events[0]).to_be(&Message::Spawn { entity }.into())?;
		expect(&events[1]).to_be(&Message::Despawn { entity }.into())?;

		Ok(())
	}

	#[test]
	fn incoming() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ReplicatePlugin);

		let _entity1 = app.world_mut().spawn(Replicate::default()).id();

		app.update();


		let mut app2 = App::new();

		app2.add_plugins(ReplicatePlugin);

		Message::loopback(app.world_mut(), app2.world_mut());

		let _dummy = app2.world_mut().spawn_empty();

		app2.update();

		let entities = app2.world().iter_entities().collect::<Vec<_>>();
		// 1 = dummy
		// 2 = replicated
		expect(entities.len()).to_be(2)?;

		Ok(())
	}
}
