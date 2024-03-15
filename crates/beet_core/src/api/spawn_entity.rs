use crate::prelude::*;
use anyhow::Result;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;

pub const ENTITY_TOPIC: &'static str = "entity";

#[derive(Resource)]
pub struct SpawnEntityHandler<T: ActionList> {
	pub send: Publisher<SpawnEntityPayload<T>>,
	pub recv: Subscriber<SpawnEntityPayload<T>>,
}

impl<T: ActionList> TopicHandler<SpawnEntityPayload<T>>
	for SpawnEntityHandler<T>
{
	fn topic() -> Topic {
		Topic::new(ENTITY_TOPIC, TopicScheme::PubSub, TopicMethod::Create)
	}
}

impl<T: ActionList> SpawnEntityHandler<T> {
	pub fn new(relay: &mut Relay) -> Result<Self> {
		Ok(Self {
			send: Self::publisher(relay)?,
			recv: Self::subscriber(relay)?,
		})
	}
}

// impl<T: ActionList> Payload for SpawnEntityPayload<T> {}

// these fields are all hacks until BevyReflect, Scene serialization etc

// This is a wip, shouldnt be so specific
pub fn handle_spawn_entity<T: ActionList>(world: &mut World) -> Result<()> {
	for message in world
		.resource_mut::<SpawnEntityHandler<T>>()
		.recv
		.try_recv_all()?
	{
		let SpawnEntityPayload {
			beet_id,
			name,
			position,
			prefab,
			position_tracking,
		} = message;

		let entity = world.spawn(Name::new(name)).id();
		let mut entity_map = world.resource_mut::<BeetEntityMap>();
		entity_map.try_insert(beet_id, entity)?;
		let mut entity = world.entity_mut(entity);
		entity.insert(beet_id);

		if position_tracking {
			entity.insert(TrackedPosition);
		}

		if let Some(pos) = position {
			entity.insert(TransformBundle {
				local: Transform::from_translation(pos),
				..default()
			});
		}
		if let Some(prefab) = prefab {
			entity.insert((ForceBundle::default(), SteerBundle {
				arrive_radius: ArriveRadius(0.2),
				wander_params: WanderParams {
					outer_distance: 0.2,
					outer_radius: 0.1,
					inner_radius: 0.01, //lower = smoother
					last_local_target: default(),
				},
				max_force: MaxForce(0.1),
				max_speed: MaxSpeed(0.3),
				..default()
			}));
			let id = entity.id();
			prefab.spawn(world, Some(id))?;
		};
	}
	Ok(())
}
