use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy_math::prelude::*;
use bevy_utils::HashMap;
use forky_bevy::extensions::Vec3Ext;
use forky_core::ResultTEExt;
use forky_web::wait_for_16_millis;
use forky_web::DocumentExt;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;

pub struct GameConfig {
	pub relay: Relay,
	pub graph: BehaviorGraph<BeeNode>,
	pub flower: bool,
}

impl Default for GameConfig {
	fn default() -> Self {
		Self {
			relay: Relay::default(),
			graph: BehaviorGraph::new(),
			flower: true,
		}
	}
}

pub struct BeeGame {
	pub relay: Relay,
	create_bee_sub: Subscriber<BehaviorGraph<BeeNode>>,
	create_flower_sub: Subscriber<Vec3>,
	despawn_sub: Subscriber<DespawnEntityPayload>,
	pub elements: HashMap<BeetEntityId, HtmlDivElement>,
}

impl BeeGame {
	pub async fn new(mut relay: Relay) -> Result<Self> {
		let create_bee_sub = create_bee_sub(&mut relay);
		let create_flower_sub = create_flower_sub(&mut relay);
		let despawn_sub = DespawnEntityHandler::subscriber(&mut relay);

		Ok(Self {
			relay,
			create_bee_sub,
			create_flower_sub,
			despawn_sub,
			elements: Default::default(),
		})
	}
	pub async fn update(&mut self) -> Result<()> {
		for payload in self.despawn_sub.try_recv_all()? {
			if let Some(id) = payload.beet_id {
				if let Some(el) = self.elements.remove(&id) {
					el.remove();
				}
			} else {
				for item in self.elements.values() {
					item.remove();
				}
				self.elements.clear();
			}
		}

		for graph in self.create_bee_sub.try_recv_all()? {
			let (id, el) = create_bee(&mut self.relay, graph).await?;
			self.elements.insert(id, el);
		}

		for pos in self.create_flower_sub.try_recv_all()? {
			let (id, el) = create_flower(&mut self.relay, pos).await?;
			self.elements.insert(id, el);
		}

		for (id, el) in &self.elements {
			if let Ok(pos) = PositionSender::subscriber(&mut self.relay, *id)
				.unwrap()
				.try_recv()
			{
				set_position(&el, pos.xy(), &get_entities_container());
			}
		}
		Ok(())
	}

	pub fn update_forever(mut self) {
		spawn_local(async move {
			loop {
				self.update().await.ok_or(|e| log::error!("{e}"));
				wait_for_16_millis().await;
			}
		});
	}
	pub fn create_bee_pub(
		relay: &mut Relay,
	) -> Publisher<BehaviorGraph<BeeNode>> {
		relay.add_publisher_with_topic(create_bee_topic()).unwrap()
	}
	pub fn create_flower_pub(relay: &mut Relay) -> Publisher<Vec3> {
		relay
			.add_publisher_with_topic(create_flower_topic())
			.unwrap()
	}
}

fn create_bee_topic() -> Topic {
	Topic::new("bee", TopicScheme::PubSub, TopicMethod::Create)
}

fn create_bee_sub(relay: &mut Relay) -> Subscriber<BehaviorGraph<BeeNode>> {
	relay
		.add_subscriber_with_topic::<BehaviorGraph<BeeNode>>(create_bee_topic())
		.unwrap()
}

async fn create_bee(
	relay: &mut Relay,
	graph: BehaviorGraph<BeeNode>,
) -> Result<(BeetEntityId, HtmlDivElement)> {
	let mut pos = Vec3::random_in_cube();
	pos.z = 0.;
	let mut create_entity = SpawnEntityHandler::requester(relay);
	let id = create_entity
		.request(
			&SpawnEntityPayload::default()
				.with_name("bee")
				.with_graph(graph)
				.with_tracked_position(pos),
		)
		.await?;
	let el = create_dom_entity("🐝", pos.xy());

	Ok((id, el))
}

fn create_flower_topic() -> Topic {
	Topic::new("flower", TopicScheme::PubSub, TopicMethod::Create)
}

fn create_flower_sub(relay: &mut Relay) -> Subscriber<Vec3> {
	relay
		.add_subscriber_with_topic(create_flower_topic())
		.unwrap()
}

async fn create_flower(
	relay: &mut Relay,
	pos: Vec3,
) -> Result<(BeetEntityId, HtmlDivElement)> {
	let mut create_entity = SpawnEntityHandler::<BeeNode>::requester(relay);
	let id = create_entity
		.request(
			&SpawnEntityPayload::default()
				.with_name("flower")
				.with_position(pos),
		)
		.await?;
	// 🥀🌹
	let el = create_dom_entity("🌻", pos.xy());
	Ok((id, el))
	// spawn_local(async move { Flower::new(relay).await.unwrap() });
	// Ok(Flower::new(relay).await?)
}

fn set_position<'a>(
	el: &HtmlElement,
	position: Vec2,
	container: &HtmlDivElement,
) {
	let container_width = container.client_width() as f32;
	let container_height = container.client_height() as f32;
	let child_width = el.client_width() as f32;
	let child_height = el.client_height() as f32;


	let left = (container_width / 2.0) + (position.x * (container_width / 2.0))
		- (child_width / 2.0);
	let top = (container_height / 2.0)
		+ (-1. * position.y * (container_height / 2.0))// invert y
		- (child_height / 2.0);

	el.set_attribute("style", &format!("left: {}px; top: {}px;", left, top))
		.unwrap();
}

fn get_entities_container() -> HtmlDivElement {
	Document::x_query_selector::<HtmlDivElement>(".entities").unwrap()
}

fn create_dom_entity(text: &str, position: Vec2) -> HtmlDivElement {
	let container = get_entities_container();
	let div = Document::x_create_div();
	div.set_inner_text(text);
	div.set_class_name("entity");
	container.append_child(&div).unwrap();
	set_position(&*div, position, &container);
	div
}
