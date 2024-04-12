use beet_esp::prelude::*;
use bevy::prelude::*;
use esp_idf_svc::hal::delay::FreeRtos;

fn main() {
	esp_idf_svc::sys::link_patches();
	esp_idf_svc::log::EspLogger::initialize_default();
	print_free("init");

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
