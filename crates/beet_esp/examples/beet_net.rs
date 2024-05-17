use beet_esp::prelude::*;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::task::block_on;


fn main() -> anyhow::Result<()> {
	init_esp()?;

	let mut wifi_client = WifiClient::new_taking_peripherals()?;
	block_on(wifi_client.connect())?;

	let (send, recv) = crossbeam_channel::unbounded();

	let mut ws_client = WsClient::new(send.clone(), recv.clone())?;


	std::thread::spawn(move || loop {
		ws_client.update().unwrap();
		FreeRtos::delay_ms(100);
	});

	loop {
		send.send(b"hello".to_vec()).unwrap();

		FreeRtos::delay_ms(100);
	}

	// Ok(())
}
