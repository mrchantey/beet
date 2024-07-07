use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::render::view::screenshot::ScreenshotManager;
use bevy::window::PrimaryWindow;
use forky_core::ResultTEExt;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Clone, Serialize, Deserialize, Event, Reflect)]
pub struct SaveScreenshot {
	pub filename: String,
}

pub fn screenshot_on_event(
	// trigger: Trigger<SaveScreenshot>,
	mut events: EventReader<SaveScreenshot>,
	main_window: Query<Entity, With<PrimaryWindow>>,
	mut screenshot_manager: ResMut<ScreenshotManager>,
) {
	let Some(event) = events.read().next() else {
		return;
	};
	screenshot_manager
		.save_screenshot_to_disk(main_window.single(), &event.filename)
		.ok_or(|e| log::error!("{e}"));
	log::info!("Saved screenshot to {}", event.filename);
}

/// Take a screenshot when ctrl+s is pressed
pub fn screenshot_on_keypress(
	// _trigger: Trigger<KeyboardInput>,
	mut events: EventReader<KeyboardInput>,
	keys: Res<ButtonInput<KeyCode>>,
	main_window: Query<Entity, With<PrimaryWindow>>,
	mut screenshot_manager: ResMut<ScreenshotManager>,
	mut counter: Local<u32>,
) {
	if events.read().count() == 0 {
		return;
	}
	if keys.any_pressed([KeyCode::ControlRight, KeyCode::ControlLeft])
		&& keys.just_pressed(KeyCode::KeyS)
	{
		#[cfg(not(target_arch = "wasm32"))]
		std::fs::create_dir_all("target/screenshots").ok();
		#[cfg(not(target_arch = "wasm32"))]
		let path = format!("target/screenshots/screenshot-{}.png", *counter);
		#[cfg(target_arch = "wasm32")]
		let path = format!("screenshot-{}.png", *counter);
		*counter += 1;
		screenshot_manager
			.save_screenshot_to_disk(main_window.single(), &path)
			.ok_or(|e| log::error!("{e}"));
		log::info!("Saved screenshot to {}", path);
	}
}
