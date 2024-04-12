use beet_esp::prelude::*;
use esp_idf_hal::task::block_on;


fn main() -> anyhow::Result<()> {
	init_esp()?;

	let mut wifi_client = WifiClient::new_taking_peripherals()?;
	block_on(wifi_client.connect())?;

	let ip_info = wifi_client.wifi.wifi().sta_netif().get_ip_info()?;

	log::info!("Wifi DHCP info: {:?}", ip_info);

	log::info!("Shutting down in 5s...");

	std::thread::sleep(core::time::Duration::from_secs(5));
	Ok(())
}
