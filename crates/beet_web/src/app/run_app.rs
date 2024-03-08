use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy_app::prelude::*;
use forky_bevy::prelude::*;
use forky_core::prelude::*;
use forky_web::wait_for_16_millis;
use wasm_bindgen_futures::spawn_local;

pub fn run(relay: Relay) {
	console_error_panic_hook::set_once();
	console_log::init_with_level(log::Level::Info).ok();
	setup_dom();
	run_app_sync(relay.clone());
	spawn_local(async move {
		BeeGame::new(relay).await.unwrap().update_forever();
	});
}




pub fn run_app_sync(relay: Relay) {
	spawn_local(async move {
		run_app(relay).await.ok_or(|e| log::error!("{e}"));
	});
}

pub async fn run_app(relay: Relay) -> Result<()> {
	let mut app = App::new();

	app.add_plugins(BeetMinimalPlugin)
		.add_plugins(BeetPlugin::<BeeNode>::new(relay.clone()));

	let _frame = app.run_on_animation_frame();

	loop {
		wait_for_16_millis().await;
	}
}
