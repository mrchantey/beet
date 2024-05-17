use super::*;
use anyhow::Result;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc;
use esp_idf_hal::ledc::LedcChannel;
use esp_idf_hal::ledc::LedcDriver;
use esp_idf_hal::ledc::LedcTimer;
use esp_idf_hal::ledc::LedcTimerDriver;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::prelude::*;


pub type DefaultHBridgeEsp<'d> = DualHBridgeEsp<'d, Gpio7, Gpio6, Gpio4, Gpio5>;

pub fn default_hbridge_esp<'d>() -> Result<DefaultHBridgeEsp<'d>> {
	let peripherals = Peripherals::take()?;

	let bridge = DualHBridgeEsp::new(
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
	Ok(bridge)
}

#[derive(Deref, DerefMut, Component)]
pub struct DualHBridgeEsp<'d, A1: Pin, A2: Pin, B1: Pin, B2: Pin>(
	pub  DualHBridge<
		PinDriver<'d, A1, Output>,
		PinDriver<'d, A2, Output>,
		LedcDriver<'d>,
		PinDriver<'d, B1, Output>,
		PinDriver<'d, B2, Output>,
		LedcDriver<'d>,
	>,
);


impl<
		'd,
		A1: Pin + OutputPin,
		A2: Pin + OutputPin,
		B1: Pin + OutputPin,
		B2: Pin + OutputPin,
	> DualHBridgeEsp<'d, A1, A2, B1, B2>
{
	pub fn new<
		PwmA: OutputPin,
		ChannelA: LedcChannel,
		TimerA: LedcTimer,
		PwmB: OutputPin,
		ChannelB: LedcChannel,
		TimerB: LedcTimer,
	>(
		in1_a: impl Peripheral<P = A1> + 'd,
		in2_a: impl Peripheral<P = A2> + 'd,
		pwm_a: impl Peripheral<P = PwmA> + 'd,
		channel_a: impl Peripheral<P = ChannelA> + 'd,
		timer_a: impl Peripheral<P = TimerA> + 'd,
		in1_b: impl Peripheral<P = B1> + 'd,
		in2_b: impl Peripheral<P = B2> + 'd,
		pwm_b: impl Peripheral<P = PwmB> + 'd,
		channel_b: impl Peripheral<P = ChannelB> + 'd,
		timer_b: impl Peripheral<P = TimerB> + 'd,
	) -> Result<DualHBridgeEsp<'d, A1, A2, B1, B2>> {
		Ok(DualHBridgeEsp(DualHBridge::new(
			PinDriver::output(in1_a)?,
			PinDriver::output(in2_a)?,
			pwm_driver(channel_a, timer_a, pwm_a)?,
			PinDriver::output(in1_b)?,
			PinDriver::output(in2_b)?,
			pwm_driver(channel_b, timer_b, pwm_b)?,
		)))
	}
	pub fn update_system(&self) -> SystemConfigs {
		todo!("bevy 0.14");
		// || {}
		// |mut motors, query| {
		// 	for value in query.iter() {
		// 		motors.from_dual_motor_value(value).unwrap();
		// 	}
		// }
	}
}

fn pwm_driver<'d, Channel: LedcChannel, Timer: LedcTimer, Pwm: OutputPin>(
	channel: impl Peripheral<P = Channel> + 'd,
	timer: impl Peripheral<P = Timer> + 'd,
	pwm: impl Peripheral<P = Pwm> + 'd,
) -> Result<LedcDriver<'d>> {
	let driver = LedcDriver::new(
		channel,
		LedcTimerDriver::new(
			timer,
			//https://electronics.stackexchange.com/questions/309056/l298n-pwm-frequency
			&ledc::config::TimerConfig::new().frequency(2.kHz().into()),
		)?,
		pwm,
	)?;
	Ok(driver)
}

// impl DualHBridgeEsp {
// 	pub fn new() -> DualHBridgeEsp { DualHBridgeEsp {} }
// }
