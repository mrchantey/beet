use crate::prelude::*;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use beet::prelude::*;
use bevy_app::prelude::*;
use bevy_math::Vec3;
use forky_bevy::prelude::*;
use forky_core::prelude::*;
use forky_web::wait_for_16_millis;
use forky_web::SearchParams;
use wasm_bindgen_futures::spawn_local;



pub struct AppOptions {
	pub initial_prefab: BehaviorPrefab<BeeNode>,
	pub bees: usize,
	pub flowers: usize,
	pub auto_flowers: Option<usize>,
	pub hide_json: bool,
}

impl Default for AppOptions {
	fn default() -> Self {
		Self {
			initial_prefab: BehaviorPrefab::from_graph(Translate::new(
				Vec3::new(-0.1, 0., 0.),
			))
			.unwrap(),
			bees: 1,
			flowers: 1,
			auto_flowers: None,
			hide_json: false,
		}
	}
}

impl AppOptions {
	pub fn from_url() -> Self {
		let mut this = Self::default();
		if let Some(bees) = SearchParams::get("bees") {
			this.bees = bees.parse().unwrap_or(1);
		}
		if let Some(flowers) = SearchParams::get("flowers") {
			this.flowers = flowers.parse().unwrap_or(1);
		}
		if let Some(auto_flowers) = SearchParams::get("auto-flowers") {
			this.auto_flowers = Some(auto_flowers.parse().unwrap_or(1));
		}
		if SearchParams::get_flag("hide-json") {
			this.hide_json = true;
		}
		if let Ok(prefab) = get_prefab_url_param() {
			this.initial_prefab = prefab;
		}
		this
	}
	pub fn with_graph<M>(mut self, graph: impl IntoBehaviorGraph<M>) -> Self {
		self.initial_prefab = graph.into_prefab().unwrap();
		self
	}

	pub fn run(&self) {
		let relay = Relay::default();
		console_error_panic_hook::set_once();
		console_log::init_with_level(log::Level::Info).ok();
		setup_ui(relay.clone(), &self).unwrap();
		run_app_sync(relay.clone());
		spawn_local(async move {
			BeeGame::new(relay).await.unwrap().update_forever();
		});
	}
}

fn get_prefab_url_param() -> Result<BehaviorPrefab<BeeNode>> {
	if let Some(tree) = SearchParams::get("graph") {
		let bytes = general_purpose::STANDARD_NO_PAD.decode(tree.as_bytes())?;
		let prefab = bincode::deserialize(&bytes)?;
		Ok(prefab)
	} else {
		anyhow::bail!("no tree param found");
	}
}


pub fn run_app_sync(relay: Relay) {
	spawn_local(async move {
		run_app(relay).await.ok_or(|e| log::error!("{e}"));
	});
}

pub async fn run_app(relay: Relay) -> Result<()> {
	let mut app = App::new();

	app.add_plugins(BeetMinimalPlugin)
		.add_plugins(SteeringPlugin::default())
		.add_plugins(BeetPlugin::<BeeNode>::new(relay.clone()));

	let _frame = app.run_on_animation_frame();

	loop {
		wait_for_16_millis().await;
	}
}
