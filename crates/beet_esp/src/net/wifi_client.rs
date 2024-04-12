// https://github.com/esp-rs/esp-idf-svc/blob/master/examples/wifi_async.rs
use super::*;
use embedded_svc::wifi::AuthMethod;
use embedded_svc::wifi::ClientConfiguration;
use embedded_svc::wifi::Configuration;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::AsyncWifi;
use esp_idf_svc::wifi::EspWifi;
use log::info;

pub struct WifiClient {
	pub wifi: AsyncWifi<EspWifi<'static>>,
}

impl WifiClient {
	/// Only do this if this is the only one using the peripherals
	pub fn new_taking_peripherals() -> anyhow::Result<Self> {
		let peripherals = Peripherals::take()?;
		let sys_loop = EspSystemEventLoop::take()?;
		Self::new(peripherals.modem, sys_loop)
	}

	pub fn new(
		modem: Modem,
		sys_loop: EspSystemEventLoop,
	) -> anyhow::Result<Self> {
		let nvs = EspDefaultNvsPartition::take()?;
		let timer_service = EspTaskTimerService::new()?;

		let wifi = AsyncWifi::wrap(
			EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
			sys_loop,
			timer_service,
		)?;

		Ok(Self { wifi })
	}

	pub async fn connect(&mut self) -> anyhow::Result<()> {
		let env = DotEnv::new();

		let wifi_configuration: Configuration =
			Configuration::Client(ClientConfiguration {
				ssid: env.heapless_ssid(),
				password: env.heapless_pass(),
				bssid: None,
				auth_method: AuthMethod::WPA2Personal,
				channel: None,
			});

		self.wifi.set_configuration(&wifi_configuration)?;

		self.wifi.start().await?;
		info!("Wifi started");

		self.wifi.connect().await?;
		info!("Wifi connected");

		self.wifi.wait_netif_up().await?;
		info!("Wifi netif up");

		Ok(())
	}
}
