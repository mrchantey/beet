use bevy::prelude::*;
// use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;

/// Struct for L298N. Two enable inputs are provided to enable or disable the device
/// independently of the input signals. The emitters of the lower transistors of each
/// bridge are connected together and the corresponding external terminal can be used
/// for the connection of an external sensing resistor. An additional supply input is
/// provided so that the logic works at a lower voltage.
#[derive(Debug, Resource)]
pub struct DualHBridge<IN1A, IN2A, PWMA, IN1B, IN2B, PWMB>
where
	IN1A: OutputPin,
	IN2A: OutputPin,
	PWMA: SetDutyCycle,
	IN1B: OutputPin,
	IN2B: OutputPin,
	PWMB: SetDutyCycle,
{
	/// motor A
	pub a: HBridge<IN1A, IN2A, PWMA>,
	/// motor B
	pub b: HBridge<IN1B, IN2B, PWMB>,
}



impl<IN1A, IN2A, PWMA, IN1B, IN2B, PWMB>
	DualHBridge<IN1A, IN2A, PWMA, IN1B, IN2B, PWMB>
where
	IN1A: OutputPin,
	IN2A: OutputPin,
	PWMA: SetDutyCycle,
	IN1B: OutputPin,
	IN2B: OutputPin,
	PWMB: SetDutyCycle,
{
	/// Creates a new `L298N` motor controller
	pub fn new(
		in1a: IN1A,
		in2a: IN2A,
		pwma: PWMA,
		in1b: IN1B,
		in2b: IN2B,
		pwmb: PWMB,
	) -> DualHBridge<IN1A, IN2A, PWMA, IN1B, IN2B, PWMB>
	where
		IN1A: OutputPin,
		IN2A: OutputPin,
		IN1B: OutputPin,
		IN2B: OutputPin,
		PWMA: SetDutyCycle,
		PWMB: SetDutyCycle,
	{
		DualHBridge {
			a: HBridge::new(in1a, in2a, pwma),
			b: HBridge::new(in1b, in2b, pwmb),
		}
	}
}

/// Struct for single bridge
#[derive(Debug)]
pub struct HBridge<IN1, IN2, PWM>
where
	IN1: OutputPin,
	IN2: OutputPin,
	PWM: SetDutyCycle,
{
	pub in1: IN1,
	pub in2: IN2,
	pub pwm: PWM,
}

impl<IN1, IN2, PWM> HBridge<IN1, IN2, PWM>
where
	IN1: OutputPin,
	IN2: OutputPin,
	PWM: SetDutyCycle,
{
	/// Creates a new single `Motor` controller
	pub fn new(in1: IN1, in2: IN2, pwm: PWM) -> HBridge<IN1, IN2, PWM>
	where
		IN1: OutputPin,
		IN2: OutputPin,
		PWM: SetDutyCycle,
	{
		// let pwm = pwm;
		// pwm.enable();

		let this = HBridge { in1, in2, pwm };
		assert_eq!(
			this.pwm.max_duty_cycle(),
			255,
			"haven't yet come across non 8 bit pwm"
		);
		this
	}

	/// Brakes the motor - Fast Motor Stop
	/// with Ven = H then C = D Fast Motor Stop
	pub fn brake(&mut self) -> &mut Self {
		self.in1.set_high().ok();
		self.in2.set_high().ok();
		self
	}

	/// Stops the motor - Free Running Motor Stop
	/// Ven = L then with C = X ; D = X
	pub fn stop(&mut self) -> &mut Self {
		self.in1.set_high().ok();
		self.in2.set_high().ok();
		self
	}

	/// Makes the motor forward direction
	/// with Ven = H then C = H ; D = L Forward
	pub fn forward(&mut self) -> &mut Self {
		self.in1.set_low().ok();
		self.in2.set_high().ok();
		self
	}

	/// Makes the motor reverse direction
	/// with Ven = H then C = L ; D = H Reverse
	pub fn reverse(&mut self) -> &mut Self {
		self.in1.set_high().ok();
		self.in2.set_low().ok();
		self
	}

	/// Returns the maximum
	pub fn get_max_duty(&self) -> u16 { self.pwm.max_duty_cycle() }

	/// Changes the motor speed, assumes
	pub fn set_duty(&mut self, duty: u8) -> Result<&mut Self, PWM::Error> {
		self.pwm.set_duty_cycle(duty as u16)?;
		Ok(self)
	}
}
