use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

fn main() -> anyhow::Result<()> {
	esp_idf_hal::sys::link_patches();

	let peripherals = Peripherals::take()?;
	// esp32-c3
	let mut led = PinDriver::output(peripherals.pins.gpio5)?;

	loop {
		led.set_high()?;
		println!("high");
		// we are sleeping here to make sure the watchdog isn't triggered
		FreeRtos::delay_ms(1000);
		println!("low");
		led.set_low()?;
		FreeRtos::delay_ms(1000);
	}
}
