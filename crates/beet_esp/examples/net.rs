use beet_esp::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;
use esp_idf_hal::delay::FreeRtos;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Default, Component, Serialize, Deserialize)]
struct MyComponent(pub i32);

fn main() -> anyhow::Result<()> {
	init_esp()?;

	// Hardware

	let mut wifi = WifiClient::new_taking_peripherals()?;
	esp_idf_hal::task::block_on(wifi.connect())?;
	let ws = WsClient::new()?;

	let mut app = App::new();

	app.insert_non_send_resource(wifi)
		// .insert_non_send_resource(ws)
		.add_plugins((
			MinimalPlugins,
			ReplicatePlugin,
			ReplicateComponentPlugin::<MyComponent>::default(),
			TransportPlugin::arc(ws),
		))
		.add_systems(Update, update)
		.finish();

	loop {
		app.update();
		// send.send(b"hello".to_vec()).unwrap();

		FreeRtos::delay_ms(100);
	}

	// Ok(())
}
fn update(query: Query<(Entity, &MyComponent), Added<MyComponent>>) {
	for (_entity, comp) in query.iter() {
		log::info!("SUCCESS - {:?}", comp);
	}
}
