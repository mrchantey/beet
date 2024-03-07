use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy_math::prelude::*;
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

pub struct BeeGame {
	pub bee: Bee,
	pub flower: Option<Flower>,
}

impl BeeGame {
	pub async fn new(mut config: GameConfig) -> Result<Self> {
		let flower = if config.flower {
			Some(Flower::new(&mut config.relay).await?)
		} else {
			None
		};
		let bee = Bee::new(&mut config.relay, config.graph).await?;
		Ok(Self { bee, flower })
	}
	pub fn update(&mut self) { self.bee.update(); }

	pub fn update_forever(mut self) {
		spawn_local(async move {
			loop {
				self.update();
				wait_for_16_millis().await;
			}
			// 🥀🌹
		});
	}
}


#[allow(unused)]
pub struct Bee {
	id: BeetEntityId,
	el: HtmlDivElement,
	position_update: Subscriber<Vec3>,
}

impl Bee {
	pub async fn new(
		relay: &mut Relay,
		graph: BehaviorGraph<BeeNode>,
	) -> Result<Self> {
		let mut create_entity = SpawnBehaviorEntityHandler::requester(relay);
		let id = create_entity
			.request(&SpawnBehaviorEntityPayload::new(
				graph,
				Some(Vec3::new(0.5, 0., 0.)),
				true,
			))
			.await?;
		let position_update = PositionSender::subscriber(relay, id)?;
		let el = create_dom_entity("🐝", Vec2::new(0.5, 0.));


		Ok(Self {
			id,
			el,
			position_update,
		})
	}
	pub fn update(&mut self) {
		if let Ok(pos) = self.position_update.try_recv() {
			set_position(&self.el, pos.xy(), &get_container());
		}
	}
}

#[allow(unused)]
pub struct Flower {
	id: BeetEntityId,
	el: HtmlDivElement,
}
impl Flower {
	pub async fn new(relay: &mut Relay) -> Result<Self> {
		let mut create_entity = SpawnEntityHandler::requester(relay);
		let id = create_entity
			.request(
				&SpawnEntityPayload::default()
					.with_position(Vec3::new(-0.5, 0., 0.)),
			)
			.await?;

		let el = create_dom_entity("🌻", Vec2::new(-0.5, 0.));

		Ok(Self { id, el })
	}
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
		+ (position.y * (container_height / 2.0))
		- (child_height / 2.0);

	el.set_attribute("style", &format!("left: {}px; top: {}px;", left, top))
		.unwrap();
}

fn get_container() -> HtmlDivElement {
	Document::x_query_selector::<HtmlDivElement>(".container").unwrap()
}

fn create_dom_entity(text: &str, position: Vec2) -> HtmlDivElement {
	let container = get_container();
	let div = Document::x_create_div();
	div.set_inner_text(text);
	div.set_class_name("entity");
	container.append_child(&div).unwrap();
	set_position(&*div, position, &container);
	div
}
