use crate::prelude::*;
use beet::prelude::*;
use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;


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
	pub fn from_dual_motor_value(
		&mut self,
		value: &DualMotorValue,
	) -> Result<(), PWMA::Error> {
		self.a.from_motor_value(&value.left)?;
		self.b
			.from_motor_value(&value.right)
			.expect("TODO: propagate this error");
		Ok(())
	}
}

impl<IN1, IN2, PWM> HBridge<IN1, IN2, PWM>
where
	IN1: OutputPin,
	IN2: OutputPin,
	PWM: SetDutyCycle,
{
	pub fn from_motor_value(
		&mut self,
		value: &MotorValue,
	) -> Result<(), PWM::Error> {
		match value.direction {
			MotorDirection::Forward => {
				self.forward();
			}
			MotorDirection::Backward => {
				self.reverse();
			}
		}
		self.set_duty(value.value)?;
		Ok(())
	}
}

// impl ApplyDualMotor for DualMotorHal
