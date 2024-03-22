use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;
use forky_bevy::extensions::Vec3Ext;
use forky_web::DocumentExt;
use web_sys::Document;
use web_sys::Element;
use web_sys::HtmlDivElement;

pub fn get_entities_container() -> HtmlDivElement {
	Document::x_query_selector::<HtmlDivElement>(".dom-sim-container").unwrap()
}


#[derive(Clone)]
pub enum DomSimMessage {
	SpawnBee,
	SpawnFlower,
	DespawnAll,
	Resize,
	SetGraph(DynGraph),
}

impl DomSimMessage {
	pub fn set_graph<M>(node: impl IntoBeetNode<M>) -> DomSimMessage {
		DomSimMessage::SetGraph(node.into_beet_node().into_graph::<BeeNode>())
	}
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
			DomSimMessage::SpawnBee => spawn_bee(world)?,
			DomSimMessage::SpawnFlower => spawn_flower(world),
			DomSimMessage::DespawnAll => {
				let mut elements =
					world.non_send_resource_mut::<DomSimElements>();
				for (entity, el) in std::mem::take(&mut elements.0).into_iter()
				{
					el.remove();
					world.despawn(entity);
				}
			}
			DomSimMessage::SetGraph(graph) => {
				*world.resource_mut::<DynGraph>() = graph;
			}
			DomSimMessage::Resize => {
				let elements =
					world.non_send_resource::<DomSimElements>().clone();
				for (entity, _) in elements.0.into_iter() {
					let mut entity = world.entity_mut(entity);
					let _changed =
						entity.get_mut::<Transform>().unwrap().as_mut();
				}
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

fn spawn_bee(world: &mut World) -> Result<()> {
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

	let graph = world.resource::<DynGraph>().clone();
	let _root = graph.spawn(world, entity)?;
	Ok(())
}

fn spawn(
	world: &mut World,
	name: impl Into<String>,
	text: impl Into<String>,
	position: Vec3,
) -> Entity {
	let parent_el = get_entities_container();
	let entity = world
		.spawn((DomSimEntity, Name::new(name.into()), TransformBundle {
			local: Transform::from_translation(position),
			..default()
		}))
		.id();

	let child_el = create_dom_entity(&parent_el, &text.into());

	world
		.non_send_resource_mut::<DomSimElements>()
		.insert(entity, child_el);
	entity
}

fn create_dom_entity(parent: &Element, text: &str) -> HtmlDivElement {
	let div = Document::x_create_div();
	div.set_inner_text(text);
	div.set_class_name("entity");
	parent.append_child(&div).unwrap();
	div
}
