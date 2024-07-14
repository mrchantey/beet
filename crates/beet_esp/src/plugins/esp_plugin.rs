use super::spawn_obstacle_avoider;
// use beet::prelude::*;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use esp_idf_hal::delay::FreeRtos;
use std::time::Duration;

#[derive(Default)]
pub struct EspPlugin;

impl Plugin for EspPlugin {
	fn build(&self, app: &mut App) {
		app /*-*/
		.add_plugins(TimePlugin)
		// .add_plugins(BeetSystemsPlugin::<CoreModule,Update>::default())
		// .add_plugins(BeetPlugins::<CoreModule>::default())
		.add_systems(Startup,spawn_obstacle_avoider)
		/*-*/;
		todo!("beet stuff");
	}
}

/// Run at 60fps at the most
pub fn run_app_with_delay(app: &mut App) -> ! {
	app.finish();
	let duration = Duration::from_millis(16);
	loop {
		app.update();
		let time = app.world().resource::<Time>();
		// let time = app.world().resource::<Time>();
		let delay_time = duration
			.checked_sub(time.delta())
			.unwrap_or(Duration::from_millis(1));
		FreeRtos::delay_ms(delay_time.as_millis() as u32);
	}
}
