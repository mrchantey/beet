use super::*;
use anyhow::Result;
use beet::prelude::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspEventLoop;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::eventloop::System;

pub struct AppHardware<'d> {
	pub hbridge: DefaultHBridgeEsp<'d>,
	pub ultrasound: DefaultUltrasoundEsp<'d>,
	pub modem: esp_idf_hal::modem::Modem,
	pub sys_loop: EspEventLoop<System>,
}


impl AppHardware<'_> {
	pub fn new() -> Result<Self> {
		let peripherals = Peripherals::take()?;
		let sys_loop = EspSystemEventLoop::take()?;
		let hbridge = DualHBridgeEsp::new(
			peripherals.pins.gpio7,
			peripherals.pins.gpio6,
			peripherals.pins.gpio16,
			peripherals.ledc.channel0,
			peripherals.ledc.timer0,
			peripherals.pins.gpio4,
			peripherals.pins.gpio5,
			peripherals.pins.gpio15,
			peripherals.ledc.channel1,
			peripherals.ledc.timer1,
		)?;

		let ultrasound = UltrasoundSensorEsp::new(
			peripherals.pins.gpio13,
			peripherals.pins.gpio12,
			DEFAULT_ULTRASOUND_MAX_DEPTH,
		)?;



		Ok(AppHardware {
			hbridge,
			ultrasound,
			modem: peripherals.modem,
			sys_loop,
		})
	}
}
