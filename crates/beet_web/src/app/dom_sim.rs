use crate::prelude::*;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use beet::prelude::*;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;
use forky_web::AnimationFrame;
use forky_web::History;
use forky_web::SearchParams;
use parking_lot::RwLock;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;

pub struct DomSim<T: ActionList> {
	pub scene: BeetSceneSerde<T>,
	pub auto_flowers: Option<Duration>,
	pub bees: usize,
	pub test_container: Option<HtmlDivElement>,
	pub flowers: usize,
	pub basic_ui: bool,
	pub phantom: PhantomData<T>,
}

impl<T: ActionList> Default for DomSim<T> {
	fn default() -> Self {
		Self {
			scene: forage().into_scene(),
			auto_flowers: None,
			basic_ui: true,
			test_container: None,
			bees: 1,
			flowers: 1,
			phantom: PhantomData,
		}
	}
}


impl<T: ActionList> DomSim<T> {
	pub fn with_node<M>(mut self, node: impl IntoBeetBuilder<M>) -> Self {
		self.scene = node.into_beet_builder().as_prefab().into_scene::<T>();
		self
	}
	pub fn with_test_container(mut self, container: HtmlDivElement) -> Self {
		self.test_container = Some(container);
		self
	}
	pub fn with_url_params(mut self) -> Self {
		if let Some(bees) = SearchParams::get("bees") {
			self.bees = bees.parse().unwrap_or(1);
		}
		if let Some(flowers) = SearchParams::get("flowers") {
			self.flowers = flowers.parse().unwrap_or(1);
		}
		if let Some(auto_flowers) = SearchParams::get("auto-flowers") {
			let val: f64 = auto_flowers.parse().unwrap_or(1.0);
			self.auto_flowers = Some(Duration::from_secs_f64(val));
		}
		if let Ok(Some(scene)) = get_scene_url_param::<T>() {
			self.scene = scene;
		}
		self
	}

	pub fn run_forever(self) -> Result<()> {
		let (send, recv) = flume::unbounded();
		self.run_with_channel(send, recv)?.forget();
		Ok(())
	}

	pub fn into_app(
		self,
		send: Sender<DomSimMessage>,
		recv: Receiver<DomSimMessage>,
	) -> Result<App> {
		for _ in 0..self.bees {
			send.send(DomSimMessage::SpawnBeeFromFirstNode)?;
		}
		for _ in 0..self.flowers {
			send.send(DomSimMessage::SpawnFlower)?;
		}

		console_error_panic_hook::set_once();
		console_log::init_with_level(log::Level::Info).ok();
		if self.basic_ui {
			setup_ui(send.clone());
		}
		let mut app = App::new();


		app /*-*/
			.add_plugins(BeetMinimalPlugin)
			.add_plugins(DefaultBeetPlugins::<T>::new())
			.insert_resource(DomSimMessageSend(send))
			.insert_resource(DomSimMessageRecv(recv))
			.add_systems(Update,(
				message_handler.pipe(log_error),
				create_elements.run_if(has_renderer),
				)
				.chain()
				.before(PreTickSet)
			)
			.add_systems(Update,(
				update_positions.run_if(has_renderer),
				despawn_elements.run_if(has_renderer),
				)
				.chain()
				.after(PostTickSet)
			)
		/*-*/;

		self.scene
			.scene
			.write_to_world(app.world_mut(), &mut Default::default())?;

		if let Some(container) = self.test_container {
			let container: &HtmlElement = &container;
			app.insert_non_send_resource(DomRenderer::new(container.clone()));
		}

		if let Some(duration) = self.auto_flowers {
			app.insert_resource(AutoFlowers(Timer::new(
				duration,
				TimerMode::Repeating,
			)))
			.add_systems(Update, auto_flowers_spawn);
		}

		Ok(app)
	}


	#[must_use]
	pub fn run_with_channel(
		self,
		send: Sender<DomSimMessage>,
		recv: Receiver<DomSimMessage>,
	) -> Result<AnimationFrame> {
		let test_container = self.test_container.is_some();

		let app = self.into_app(send, recv)?;
		let app = Arc::new(RwLock::new(app));

		if test_container {
			test_container_listener(app.clone());
		}

		let frame = AnimationFrame::new(move || {
			app.try_write().map(|mut a| a.update());
		});

		Ok(frame)
	}
}
const SCENE_PARAM: &str = "scene";

pub fn get_scene_url_param<T: ActionTypes>() -> Result<Option<BeetSceneSerde<T>>>
{
	if let Some(tree) = SearchParams::get(SCENE_PARAM) {
		let bytes = general_purpose::STANDARD_NO_PAD.decode(tree.as_bytes())?;
		let scene: BeetSceneSerde<T> = bincode::deserialize(&bytes)?;
		Ok(Some(scene))
	} else {
		Ok(None)
	}
}

const MAX_URL_LENGTH: usize = 1900;
pub fn set_scene_url_param<T: ActionTypes>(world: &World) -> Result<()> {
	let serde = BeetSceneSerde::<T>::new(world);
	let val = bincode::serialize(&serde)?;
	let val = general_purpose::STANDARD_NO_PAD.encode(val);
	if val.len() > MAX_URL_LENGTH {
		anyhow::bail!(
			"graph base64 length is too long: {} > {}",
			val.len(),
			MAX_URL_LENGTH
		);
	}
	History::set_param(SCENE_PARAM, &val);
	Ok(())
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
