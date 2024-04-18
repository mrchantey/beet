use beet_esp::prelude::*;
use esp_idf_hal::delay::Delay;

fn main() -> anyhow::Result<()> {
	init_esp()?;

	let mut ultrasound = default_ultrasound_esp()?;

	loop {
		let distance = ultrasound.measure_or_max();
		println!("Dist: {:.2}", distance);
		Delay::default().delay_ms(16);
	}
}
