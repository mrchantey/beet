use beet_esp::prelude::*;
use beet_net::exports::lightyear::client::plugin::ClientPlugins;
use beet_net::exports::lightyear_common::apps::default_settings;
use beet_net::exports::lightyear_common::apps::settings_to_client_config_crossbeam;
use beet_net::prelude::*;
use bevy::prelude::*;
use esp_idf_hal::task::block_on;


fn main() -> anyhow::Result<()> {
	init_esp()?;

	// Hardware
	let mut wifi_client = WifiClient::new_taking_peripherals()?;
	block_on(wifi_client.connect())?;

	let (send, recv) = crossbeam_channel::unbounded();

	std::thread::spawn(move || {
		WsClient::new(send.clone(), recv.clone())?.run()
	});

	let settings = default_settings();

	let (send, recv) = crossbeam_channel::unbounded();

	let config =
		settings_to_client_config_crossbeam(settings, send.clone(), recv, None);

	let mut app = App::new();
	app.add_plugins((
		ClientPlugins { config },
		BaseClientPlugin,
		BaseSharedPlugin,
		DefaultPlugins,
	))
	.finish();

	// loop {
	// 	send.send(b"hello".to_vec()).unwrap();

	// 	FreeRtos::delay_ms(100);
	// }

	Ok(())
}
