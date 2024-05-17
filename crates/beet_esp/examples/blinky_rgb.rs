//! A simple example to change colours of a WS2812/NeoPixel compatible LED.
//!
//! This example demonstrates the use of [`FixedLengthSignal`][crate::rmt::FixedLengthSignal] which
//! lives on the stack and requires a known length before creating it.
//!
//! There is a similar implementation in the esp-idf project:
//! https://github.com/espressif/esp-idf/tree/20847eeb96/examples/peripherals/rmt/led_strip
//!
//! Datasheet (PDF) for a WS2812, which explains how the pulses are to be sent:
//! https://cdn-shop.adafruit.com/datasheets/WS2812.pdf

use beet_esp::prelude::*;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::*;


fn main() -> anyhow::Result<()> {
	init_esp()?;

	// Onboard RGB LED pin
	let peripherals = Peripherals::take()?;

	// ESP32-C3-DevKitC-02 gpio8,
	// ESP32-C3-DevKit-RUST-1 gpio2
	// ESP32-S3 AI gpio48
	let led = peripherals.pins.gpio0;
	let channel = peripherals.rmt.channel0;
	let config = TransmitConfig::new().clock_divider(1);
	let mut tx = TxRmtDriver::new(channel, led, &config)?;

	log::info!("white");
	// 3 seconds white at 10% brightness
	Rgb::new(255, 255, 255).transmit(&mut tx)?;
	FreeRtos::delay_ms(3000);

	log::info!("rainbow");
	// infinite rainbow loop at 20% brightness
	(0..360).cycle().try_for_each(|hue| {
		FreeRtos::delay_ms(10);
		Rgb::from_hsv(hue, 100, 20)?.transmit(&mut tx)
	})
}
