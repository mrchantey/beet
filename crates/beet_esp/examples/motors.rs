use beet_esp::prelude::*;
use std::time::Duration;
// use esp32c3_hal::IO::*;

fn main() -> anyhow::Result<()> {
	init_esp()?;

	let mut bridge = default_hbridge_esp()?;
	println!("awake");
	bridge.a.stop();
	bridge.b.stop();
	std::thread::sleep(Duration::from_secs(1));

	println!("starting");
	bridge.a.forward().set_duty(255)?;
	bridge.b.forward().set_duty(255)?;

	std::thread::sleep(Duration::from_secs(3));

	println!("stopping");
	bridge.a.stop();
	bridge.b.stop();


	Ok(())
}
