use anyhow::Result;
use esp_idf_hal::gpio::OutputPin;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::RmtChannel;
use esp_idf_hal::rmt::TxRmtDriver;


pub struct EspDevice<'d> {
	pub tx_rmt: TxRmtDriver<'d>,
	pub peripherals: Peripherals,
}


impl<'d> EspDevice<'d> {
	/// Wrapper for esp devices, includes rmt transmitter and clock
	pub fn new<C: RmtChannel>(
		channel: impl Peripheral<P = C> + 'd,
		led_pin: impl Peripheral<P = impl OutputPin> + 'd,
	) -> Result<Self> {
		esp_idf_svc::sys::link_patches();
		// esp_idf_svc::log::EspLogger::initialize_default();
		let config = TransmitConfig::new().clock_divider(1);
		let tx_rmt = TxRmtDriver::new(channel, led_pin, &config)?;
		let peripherals = Peripherals::take()?;



		Ok(Self {
			peripherals,
			tx_rmt,
		})
	}
}
