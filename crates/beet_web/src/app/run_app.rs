use anyhow::Result;
use beet::action_list;
use beet::prelude::*;
use bevy_math::Vec2;
use forky_core::prelude::*;
use wasm_bindgen_futures::spawn_local;


pub struct EntityToSpawn {
	pub text: String,
	pub position: Vec2,
	// pub graph: Option<BehaviorGraph<MyActions>>,
}


pub fn run_app_sync(relay: Relay) {
	spawn_local(async move {
		run_app(relay).await.ok_or(|e| log::error!("{e}"));
	});
}


pub async fn run_app(relay: Relay) -> Result<()> {
	console_error_panic_hook::set_once();
	console_log::init_with_level(log::Level::Info).ok();




	Ok(())
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
