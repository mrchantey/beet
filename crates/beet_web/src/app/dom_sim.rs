use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;
use forky_core::ResultTEExt;
use forky_web::AnimationFrame;
use forky_web::SearchParams;
use parking_lot::RwLock;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;

pub struct DomSim<T: ActionList> {
	pub scene: BeetSceneSerde<T>,
	pub scene_url: Option<String>,
	pub auto_flowers: Option<Duration>,
	pub bees: usize,
	pub test_container: Option<HtmlDivElement>,
	pub flowers: usize,
	pub phantom: PhantomData<T>,
}

impl<T: ActionList> Default for DomSim<T> {
	fn default() -> Self {
		Self {
			scene: forage().into_scene(),
			scene_url: None,
			auto_flowers: None,
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
		if let Some(scene_url) = SearchParams::get("scene") {
			self.scene_url = Some(scene_url);
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
	) -> Result<Arc<RwLock<App>>> {
		for _ in 0..self.bees {
			send.send(DomSimMessage::SpawnBeeFromFirstNode)?;
		}
		for _ in 0..self.flowers {
			send.send(DomSimMessage::SpawnFlower)?;
		}

		console_error_panic_hook::set_once();
		console_log::init_with_level(log::Level::Info).ok();

		let mut app = App::new();


		app /*-*/
			.add_plugins(BeetMinimalPlugin)
			.add_plugins(DefaultBeetPlugins::<T>::new())
			.insert_resource(DomSimMessageSend(send.clone()))
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

		app.add_systems(Update, auto_flowers_spawn);


		if let Some(container) = self.test_container {
			let container: &HtmlElement = &container;
			app.insert_non_send_resource(DomRenderer::new(container.clone()));
		}

		if let Some(duration) = self.auto_flowers {
			app.world_mut()
				.spawn(AutoFlowers(Timer::new(duration, TimerMode::Repeating)));
		}

		let app = Arc::new(RwLock::new(app));

		if let Some(url) = self.scene_url {
			try_load_url_scene(app.clone(), url, self.scene)
		} else {
			let mut app = app.write();
			self.scene
				.scene
				.write_to_world(app.world_mut(), &mut Default::default())?;
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

		let app = self.into_app(send.clone(), recv)?;

		if test_container {
			setup_ui(send, app.clone());
			// test_container_listener(app.clone());
		}

		let frame = AnimationFrame::new(move || {
			app.try_write().map(|mut a| a.update());
		});

		Ok(frame)
	}
}



fn try_load_url_scene<T: ActionList>(
	app: Arc<RwLock<App>>,
	url: String,
	fallback: BeetSceneSerde<T>,
) {
	let app = app.clone();
	spawn_local(async move {
		if let Err(e) = try_load_url_scene_inner::<T>(app.clone(), url).await {
			log::error!(
				"Failed to load scene, loading default instead...\n{e}"
			);
			let mut app = app.write();
			fallback
				.scene
				.write_to_world(app.world_mut(), &mut Default::default())
				.ok_or(|e| log::error!("{e}"));
		};
	});
}


async fn try_load_url_scene_inner<T: ActionList>(
	app: Arc<RwLock<App>>,
	url: String,
) -> Result<()> {
	let scene = fetch_scene::<T>(&url).await?;

	let mut app = app.write();
	let mut world = app.world_mut();
	scene
		.scene
		.write_to_world(&mut world, &mut Default::default())?;
	Ok(())
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
