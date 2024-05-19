use beet_esp::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use esp_idf_hal::delay::FreeRtos;

fn main() -> anyhow::Result<()> {
	init_esp()?;

	// Hardware

	let mut wifi = WifiClient::new_taking_peripherals()?;
	esp_idf_hal::task::block_on(wifi.connect())?;
	let ws = WsClient::new()?;

	let mut app = App::new();

	app.insert_non_send_resource(wifi)
		.insert_non_send_resource(ws)
		.add_plugins((
			DefaultPlugins.build().disable::<LogPlugin>(),
			RepliconEspClientPlugin,
		))
		.finish();

	loop {
		app.update();
		// send.send(b"hello".to_vec()).unwrap();

		FreeRtos::delay_ms(100);
	}

	// Ok(())
}
