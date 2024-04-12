use embedded_hal::delay::DelayNs;
use embedded_hal::digital::InputPin;
use embedded_hal::digital::OutputPin;
use esp_idf_hal::delay::FreeRtos;
use std::time::Duration;
use std::time::Instant;
/// Speed of sound in meters per second
const SPEED_OF_SOUND: f32 = 343.;
pub const DEFAULT_ULTRASOUND_MAX_DEPTH: f32 = 1.5;

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
		let timeout = calculate_timeout(max_depth);

		UltrasoundSensor {
			trig,
			echo,
			max_depth,
			timeout,
		}
	}

	pub fn measure(&mut self) -> Option<f32> {
		self.trig.set_low().unwrap();
		// std::thread::sleep(Duration::from_micros(2));
		FreeRtos::delay_us(&mut FreeRtos, 2);
		//start pulse
		self.trig.set_high().unwrap();
		std::thread::sleep(Duration::from_micros(10));
		FreeRtos::delay_us(&mut FreeRtos, 10);
		//stop pulse
		self.trig.set_low().unwrap();

		if let Some(duration) = pulse_in(&mut self.echo, true, self.timeout) {
			let distance = duration.as_secs_f32() * SPEED_OF_SOUND / 2.;
			Some(distance)
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



/// distance to time of flight
fn calculate_timeout(dist_meters: f32) -> Duration {
	let secs = dist_meters / SPEED_OF_SOUND * 2.;
	Duration::from_millis((secs * 1000.) as u64)
}


/// Set to true to read a high pulse
fn pulse_in(
	pin: &mut impl InputPin,
	value: bool,
	timeout: Duration,
) -> Option<Duration> {
	let start_time = Instant::now();
	while pin.is_high().unwrap() != value {
		if start_time.elapsed() > timeout {
			return None;
		}
	}
	let pulse_start_time = Instant::now();
	while pin.is_high().unwrap() == value {
		if start_time.elapsed() > timeout {
			return None;
		}
	}
	Some(pulse_start_time.elapsed())
}
