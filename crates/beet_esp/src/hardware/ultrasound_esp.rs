use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;

pub type DefaultUltrasoundEsp<'d> = UltrasoundSensorEsp<'d, Gpio13, Gpio12>;

pub fn default_ultrasound_esp<'d>() -> Result<DefaultUltrasoundEsp<'d>> {
	let peripherals = Peripherals::take()?;

	let ultrasound = UltrasoundSensorEsp::new(
		peripherals.pins.gpio13,
		peripherals.pins.gpio12,
		DEFAULT_ULTRASOUND_MAX_DEPTH,
	)?;
	Ok(ultrasound)
}

#[derive(Deref, DerefMut, Component)]
pub struct UltrasoundSensorEsp<'d, Trig: Pin, Echo: Pin>(
	pub  UltrasoundSensor<
		PinDriver<'d, Trig, Output>,
		PinDriver<'d, Echo, Input>,
	>,
);

impl<'d, Trig: Pin + OutputPin, Echo: Pin + InputPin>
	UltrasoundSensorEsp<'d, Trig, Echo>
{
	pub fn new(
		trig: impl Peripheral<P = Trig> + 'd,
		echo: impl Peripheral<P = Echo> + 'd,
		max_depth: Meters,
	) -> Result<UltrasoundSensorEsp<'d, Trig, Echo>> {
		Ok(Self(UltrasoundSensor::new(
			PinDriver::output(trig)?,
			PinDriver::input(echo)?,
			max_depth,
		)))
	}

	//TODO only update when sensor actions active
	pub fn update_system(&self) -> SystemConfigs {
		todo!("bevy 0.14");
		// 	move |mut smoother, mut sensor, mut query| {
		// 		for mut value in query.iter_mut() {
		// 			let new_depth = sensor.measure_or_max();
		// 			let smoothed = smoother.add_and_smooth(new_depth);

		// 			if value.set_if_neq(DepthValue(Some(smoothed))) {
		// 				log::info!(
		// 					"New depth: {:.2}",
		// 					smoothed
		// 				);
		// 			}
		// 		}
		// 	}
	}
}
