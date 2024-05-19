use crate::prelude::*;
use bevy::prelude::*;

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
				),
			);
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::*;

	fn get_events(
		mut outgoing: EventReader<MessageOutgoing>,
	) -> Vec<MessageOutgoing> {
		outgoing.read().cloned().collect()
	}

	#[test]
	fn outgoing() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ReplicatePlugin);

		let entity = app.world_mut().spawn(Replicate::default()).id();

		app.update();
		let events = app.world_mut().run_system_once(get_events);
		expect(events.len()).to_be(1)?;
		expect(&events[0]).to_be(&Message::Spawn { entity }.into())?;

		app.world_mut().despawn(entity);

		app.update();
		let events = app.world_mut().run_system_once(get_events);
		expect(events.len()).to_be(2)?;
		expect(&events[1]).to_be(&Message::Despawn { entity }.into())?;

		Ok(())
	}

	#[test]
	fn incoming() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ReplicatePlugin);

		let _entity1 = app.world_mut().spawn(Replicate::default()).id();

		app.update();
		let events = app.world_mut().run_system_once(get_events);
		let events = events
			.into_iter()
			.map(|e| e.into())
			.collect::<Vec<MessageIncoming>>();


		let mut app2 = App::new();

		app2.add_plugins(ReplicatePlugin);

		app2.world_mut().spawn_empty();

		for event in events {
			app2.world_mut().send_event(event);
		}
		app2.update();

		let entities = app2.world().iter_entities().collect::<Vec<_>>();
		expect(entities.len()).to_be(2)?;

		Ok(())
	}
}
