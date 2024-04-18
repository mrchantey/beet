use beet_esp::prelude::*;
use esp_idf_hal::delay::FreeRtos;
use std::time::Instant;


fn main() -> anyhow::Result<()> {
	init_esp()?;

	let mut ultrasound = default_ultrasound_esp()?;
	let buffer_size = 50;
	let mut smoother = SignalSmoother::new(buffer_size); // Adjust this value to change the smoothing factor

	let mut last_print = Instant::now();

	loop {
		let distance = ultrasound.measure_or_max();
		let smoothed_distance = smoother.add_and_smooth(distance);

		let now = Instant::now();
		if now.duration_since(last_print).as_millis() > 100 {
			println!("Dist: {:.2}, Smooth: {:.2}", distance, smoothed_distance);
			last_print = now;
		}
		FreeRtos::delay_ms(16);
	}
}
