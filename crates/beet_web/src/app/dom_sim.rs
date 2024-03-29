use crate::prelude::*;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use beet::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::utils::HashMap;
use flume::Receiver;
use flume::Sender;
use forky_web::AnimationFrame;
use forky_web::History;
use forky_web::SearchParams;
use std::marker::PhantomData;
use std::time::Duration;
use web_sys::HtmlDivElement;

#[derive(Component)]
pub struct DomSimEntity;

#[derive(Clone, Default, Deref, DerefMut)]
pub struct DomSimElements(pub HashMap<Entity, HtmlDivElement>);

pub struct DomSim<T: ActionList> {
	pub scene: DynamicScene,
	pub auto_flowers: Option<Duration>,
	pub bees: usize,
	pub flowers: usize,
	pub basic_ui: bool,
	pub phantom: PhantomData<T>,
}

impl<T: ActionList> Default for DomSim<T> {
	fn default() -> Self {
		Self {
			scene: forage().into_scene::<T>(),
			auto_flowers: None,
			basic_ui: true,
			bees: 1,
			flowers: 1,
			phantom: PhantomData,
		}
	}
}


impl<T: ActionList> DomSim<T> {
	pub fn with_node<M>(mut self, node: impl IntoBeetBuilder<M>) -> Self {
		let node = node.into_beet_builder();
		self.scene = node.into_scene::<T>();
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
			.insert_non_send_resource(DomSimElements::default())
			.add_systems(Update,(
				message_handler.pipe(log_error),
				update_positions
			).chain()
			)
			.add_systems(Update, despawn_elements)
		/*-*/;

		self.scene
			.write_to_world(&mut app.world, &mut Default::default())?;


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
		let mut app = self.into_app(send, recv)?;
		let frame = AnimationFrame::new(move || {
			app.update();
		});

		Ok(frame)
	}
}

pub fn get_scene_url_param<T: ActionTypes>() -> Result<Option<DynamicScene>> {
	if let Some(tree) = SearchParams::get("scene") {
		let bytes = general_purpose::STANDARD_NO_PAD.decode(tree.as_bytes())?;
		let scene: BeetSceneSerde<T> = bincode::deserialize(&bytes)?;
		Ok(Some(scene.scene))
	} else {
		Ok(None)
	}
}


const MAX_URL_LENGTH: usize = 1900;
pub fn set_scene_url_param<T: ActionTypes>(world: &World) -> Result<()> {
	let scene = DynamicScene::from_world(world);
	let serde = BeetSceneSerde::<T>::new(scene);
	let val = bincode::serialize(&serde).unwrap();
	let val = general_purpose::STANDARD_NO_PAD.encode(val);
	if val.len() > MAX_URL_LENGTH {
		anyhow::bail!(
			"graph base64 length is too long: {} > {}",
			val.len(),
			MAX_URL_LENGTH
		);
	}
	History::set_param("graph", &val);
	Ok(())
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
