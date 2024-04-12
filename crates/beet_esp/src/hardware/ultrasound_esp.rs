use super::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;

pub type DefaultUltrasoundEsp<'d> = UltrasoundSensorEsp<'d, Gpio1, Gpio0>;

pub fn default_ultrasound_esp<'d>() -> Result<DefaultUltrasoundEsp<'d>> {
	let peripherals = Peripherals::take()?;

	let ultrasound = UltrasoundSensorEsp::new(
		peripherals.pins.gpio1,
		peripherals.pins.gpio0,
		DEFAULT_ULTRASOUND_MAX_DEPTH,
	)?;
	Ok(ultrasound)
}

#[derive(Deref, DerefMut, Component)]
pub struct UltrasoundSensorEsp<'d, T: Pin, E: Pin>(
	pub UltrasoundSensor<PinDriver<'d, T, Output>, PinDriver<'d, E, Input>>,
);

impl<'d, T: Pin + OutputPin, E: Pin + InputPin> UltrasoundSensorEsp<'d, T, E> {
	pub fn new(
		trigger: impl Peripheral<P = T> + 'd,
		echo: impl Peripheral<P = E> + 'd,
		max_depth: Meters,
	) -> Result<UltrasoundSensorEsp<'d, T, E>> {
		Ok(UltrasoundSensorEsp(UltrasoundSensor::new(
			PinDriver::output(trigger)?,
			PinDriver::input(echo)?,
			max_depth,
		)))
	}

	//TODO only update when sensor actions active
	pub fn update_system(
		&self,
	) -> impl Fn(NonSendMut<Self>, Query<&mut DepthSensor>) {
		move |mut sensor, mut query| {
			for mut depth_value in query.iter_mut() {
				// if depth_value.max_depth != self.max_depth {
				// 	depth_value.max_depth = self.max_depth;
				// }
				if let Some(depth) = sensor.measure() {
					depth_value.value = depth;
				} else {
					depth_value.value = depth_value.max_depth;
				}
			}
		}
	}
}
