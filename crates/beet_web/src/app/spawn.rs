use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;
use forky_bevy::extensions::Vec3Ext;




#[derive(Clone)]
pub enum DomSimMessage {
	SpawnBee(BeetNode),
	SpawnBeeFromFirstNode,
	SpawnFlower,
	DespawnAll,
	Resize,
}

#[derive(Resource, Deref, DerefMut)]
pub struct DomSimMessageRecv(pub Receiver<DomSimMessage>);
#[derive(Resource, Deref, DerefMut)]
pub struct DomSimMessageSend(pub Sender<DomSimMessage>);

pub fn message_handler(world: &mut World) -> Result<()> {
	let Ok(messages) = world.resource_mut::<DomSimMessageRecv>().try_recv_all()
	else {
		return Ok(()); // disconnected
	};

	for message in messages {
		match message {
			DomSimMessage::SpawnBeeFromFirstNode => {
				match BeetNode::get_roots(world).first() {
					Some(node) => spawn_bee(world, *node)?,
					None => {
						log::error!("SpawnBeeFromFirstNode - no node found")
					}
				}
			}
			DomSimMessage::SpawnBee(node) => spawn_bee(world, node)?,
			DomSimMessage::SpawnFlower => spawn_flower(world),
			DomSimMessage::DespawnAll => {
				// dont despawn everything, we need the graph
				for entity in world
					.query_filtered::<Entity, With<Transform>>()
					.iter(world)
					.collect::<Vec<_>>()
				{
					world.despawn(entity);
				}
			}
			DomSimMessage::Resize => {
				trigger_transform_change(world);
			}
		}
	}
	Ok(())
}


fn spawn_flower(world: &mut World) {
	let mut position = Vec3::random_in_cube();
	position.z = 0.;
	position.y = position.y * 0.5 - 0.5;
	spawn(world, "flower", "🌻", position);
}

fn spawn_bee(world: &mut World, node: BeetNode) -> Result<()> {
	let mut position = Vec3::random_in_cube();
	position.z = 0.;
	let entity = spawn(world, "bee", "🐝", position);

	world
		.entity_mut(entity)
		.insert((ForceBundle::default(), SteerBundle {
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

	let new_node = node.deep_clone(world)?;
	new_node.bind_agent(world, entity);

	Ok(())
}

fn spawn(
	world: &mut World,
	name: impl Into<String>,
	text: impl Into<String>,
	position: Vec3,
) -> Entity {
	let entity = world
		.spawn((
			Name::new(name.into()),
			RenderText(text.into()),
			TransformBundle {
				local: Transform::from_translation(position),
				..default()
			},
		))
		.id();
	entity
}
