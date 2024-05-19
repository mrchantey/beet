use beet_esp::prelude::*;
use bevy::prelude::*;

pub fn main() -> anyhow::Result<()> {
	init_esp()?;

	let AppHardware {
		hbridge,
		ultrasound,
		// modem,
		// sys_loop,
		..
	} = AppHardware::new()?;

	let mut app = App::new();
	app /*-*/
		.add_plugins(EspPlugin::default())
		// depth sensor
		.add_systems(PreUpdate, ultrasound.update_system())
		.insert_non_send_resource(ultrasound)
		// motors
		.add_systems(PostUpdate, hbridge.update_system())
		.insert_non_send_resource(hbridge)
	/*-*/;


	// let mut wifi = WifiClient::new(modem,sys_loop)?;
	// esp_idf_hal::task::block_on(wifi.connect())?;
	// let ws = WsClient::new()?;
	// app.insert_non_send_resource(wifi)
	// 	.insert_non_send_resource(ws);


	run_app_with_delay(&mut app);
}
