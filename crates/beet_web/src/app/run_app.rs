use crate::dom::run_dom_sync;
use anyhow::Result;
use beet::action_list;
use beet::prelude::*;
use bevy_app::prelude::*;
use bevy_math::Vec2;
use forky_bevy::prelude::*;
use forky_core::prelude::*;
use forky_web::wait_for_16_millis;
use wasm_bindgen_futures::spawn_local;

pub struct EntityToSpawn {
	pub text: String,
	pub position: Vec2,
	// pub graph: Option<BehaviorGraph<MyActions>>,
}

pub fn run(graph: BehaviorGraph<CoreNode>) {
	let relay = Relay::default();
	run_app_sync(relay.clone());
	run_dom_sync(relay, graph);
}

pub fn run_app_sync(relay: Relay) {
	spawn_local(async move {
		run_app(relay).await.ok_or(|e| log::error!("{e}"));
	});
}

pub async fn run_app(relay: Relay) -> Result<()> {
	console_error_panic_hook::set_once();
	console_log::init_with_level(log::Level::Info).ok();

	let mut app = App::new();

	app.add_plugins(BeetMinimalPlugin)
		.add_plugins(BeetPlugin::<CoreNode>::new(relay.clone()));

	let _frame = app.run_on_animation_frame();

	loop {
		wait_for_16_millis().await;
	}
}


action_list!(MyActions, [
	//builtin
	EmptyAction,
	SetRunResult,
	SetScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);
