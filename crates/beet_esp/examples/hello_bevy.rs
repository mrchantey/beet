use beet_esp::prelude::*;
use bevy::prelude::*;
use esp_idf_svc::hal::delay::FreeRtos;

fn main()->anyhow::Result<()> {
	init_esp()?;

	let mut app = App::new();

	app.add_plugins(bevy::time::TimePlugin);
	app.add_systems(Update, |time: Res<Time>| {
		log::info!("the time is {}", time.elapsed_seconds_f64())
	});
	loop {
		FreeRtos::delay_ms(16);
		app.update();
	}
}
