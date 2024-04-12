use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use std::time::Duration;
use std::time::Instant;


pub struct AppPlugin;

impl Plugin for AppPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(TimePlugin);
		app.add_plugins(VehiclePlugin);
		app.add_systems(Update, spawn_vehicle);
	}
}

pub fn run_app_with_delay(app: &mut App) {
	app.finish();
	loop {
		app.update();
		delay(&app, Duration::from_millis(16));
	}
}

/// Attempt to accurately delay the given duration, offset by actual frame duration.
fn delay(app: &App, duration: Duration) {
	let time = app.world.resource::<Time>();

	if let Some(delay_time) = duration.checked_sub(time.delta()) {
		// log::info!(
		// 	"frame_duration: {:?}, delay_time: {:?}",
		// 	frame_duration, delay_time
		// );
		esp_idf_hal::delay::FreeRtos::delay_ms(delay_time.as_millis() as u32);
	}
}
