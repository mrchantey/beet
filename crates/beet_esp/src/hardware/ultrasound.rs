use beet::prelude::*;
use embedded_hal::digital::InputPin;
use embedded_hal::digital::OutputPin;
use esp_idf_hal::delay::Ets;
use std::time::Duration;
use std::time::Instant;


/// Speed of sound in meters per second
const SPEED_OF_SOUND: f32 = 343.;

pub type Meters = f32;


pub struct UltrasoundSensor<Trig, Echo>
where
	Trig: OutputPin,
	Echo: InputPin,
{
	pub trig: Trig,
	pub echo: Echo,
	/// max depth in meters
	pub max_depth: Meters,
	pub timeout: Duration,
}

impl<Trig, Echo> UltrasoundSensor<Trig, Echo>
where
	Trig: OutputPin,
	Echo: InputPin,
{
	pub fn new(
		trig: Trig,
		echo: Echo,
		// usually only get to about 60-75 percent of this
		max_depth: Meters,
	) -> UltrasoundSensor<Trig, Echo> {
		let timeout = meters_to_tof(max_depth);
		log::info!("Timeout: {:?}", timeout);

		let mut this = UltrasoundSensor {
			trig,
			echo,
			max_depth,
			timeout,
		};
		this.trig.set_low().unwrap();

		this
	}

	pub fn measure(&mut self) -> Option<f32> {
		// clear signal
		self.trig.set_low().unwrap();
		Ets::delay_us(5);
		//start pulse
		self.trig.set_high().unwrap();
		Ets::delay_us(10);
		//stop pulse
		self.trig.set_low().unwrap();

		if let Some(duration) = pulse_in(&mut self.echo, true, self.timeout) {
			Some(tof_to_meters(duration))
		} else {
			None
		}
	}
	pub fn measure_or_max(&mut self) -> f32 {
		match self.measure() {
			Some(distance) => distance,
			None => DEFAULT_ULTRASOUND_MAX_DEPTH,
		}
	}
}

fn meters_to_tof(dist_meters: f32) -> Duration {
	Duration::from_secs_f32(dist_meters / SPEED_OF_SOUND * 2.)
}
fn tof_to_meters(duration: Duration) -> f32 {
	duration.as_secs_f32() * SPEED_OF_SOUND / 2.
}

/// Set to true to read a high pulse
fn pulse_in(
	pin: &mut impl InputPin,
	value: bool,
	timeout: Duration,
) -> Option<Duration> {
	let start_time = Instant::now();
	// await the start of the pulse
	while pin.is_high().unwrap() != value {
		if start_time.elapsed() > timeout {
			return None;
		}
	}
	let pulse_start_time = Instant::now();
	// await the end of the pulse
	while pin.is_high().unwrap() == value {
		if start_time.elapsed() > timeout {
			return None;
		}
	}
	Some(pulse_start_time.elapsed())
}
