//! Typed physical quantities shared across the framework.
//!
//! Each is a single-field `f32` newtype with a *hidden* inner unit (like
//! [`core::time::Duration`]), so callers must go through the named constructors
//! and accessors and can never confuse, say, degrees for radians. A robot API or a
//! `SetDrive` action takes and returns these rather than bare `f32`; a transport layer
//! converts them to its wire's fixed units at the encode boundary only.
//!
//! A driven body carries its commanded [`LinearVelocity`] + [`AngularVelocity`]
//! inside a `DifferentialDrive` component (in `beet_action`); the units themselves are
//! plain values, not components. The cross-unit ratios mirror `arduino-alvik`'s
//! `conversions.py`.

use crate::prelude::*;
use core::f32::consts::PI;
use core::f32::consts::TAU;
use core::ops::Add;
use core::ops::Mul;
use core::ops::Neg;
use core::ops::Sub;

/// An angle. Inner unit is **radians**.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub struct Angle(f32);

impl Angle {
	/// An angle of `radians` radians.
	pub fn from_radians(radians: f32) -> Self { Self(radians) }
	/// An angle of `degrees` degrees.
	pub fn from_degrees(degrees: f32) -> Self { Self(degrees * PI / 180.0) }
	/// An angle of `revolutions` full turns.
	pub fn from_revolutions(revolutions: f32) -> Self {
		Self(revolutions * TAU)
	}
	/// An angle as a percent of a revolution (`1% = 3.6°`, matching
	/// `conversions.py`).
	pub fn from_percent(percent: f32) -> Self { Self(percent * 0.01 * TAU) }

	/// This angle in radians.
	pub fn as_radians(self) -> f32 { self.0 }
	/// This angle in degrees.
	pub fn as_degrees(self) -> f32 { self.0 * 180.0 / PI }
	/// This angle in full revolutions.
	pub fn as_revolutions(self) -> f32 { self.0 / TAU }
	/// This angle as a percent of a revolution.
	pub fn as_percent(self) -> f32 { self.0 / TAU * 100.0 }
}

/// An angular velocity. Inner unit is **degrees per second** — the unit the robot
/// wire and a markup `angular=90` both use, so a bare number coerces correctly (the
/// reflect path sets the stored field, and the stored field *is* deg/s).
///
/// A plain value, not a component: a driven body carries its commanded turn rate
/// inside a `DifferentialDrive` component, not as a bare `AngularVelocity`.
///
/// The `%`-of-max forms are context-dependent (`MOTOR_MAX_RPM` for a wheel,
/// `ROBOT_MAX_DEG_S` for the robot), so they live on the robot/wheel helpers,
/// not here.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// serialize as a plain number (deg/s), not a `{ 0: .. }` newtype wrapper, so a
// model authors `angular=90` and the robot wire reads one bare value.
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct AngularVelocity(f32);

impl AngularVelocity {
	/// A rate of `deg_per_sec` degrees per second.
	pub fn from_deg_per_sec(deg_per_sec: f32) -> Self { Self(deg_per_sec) }
	/// A rate of `rad_per_sec` radians per second.
	pub fn from_rad_per_sec(rad_per_sec: f32) -> Self {
		Self(rad_per_sec * 180.0 / PI)
	}
	/// A rate of `rpm` revolutions per minute (`1 rpm = 6 deg/s`).
	pub fn from_rpm(rpm: f32) -> Self { Self(rpm * 6.0) }
	/// A rate of `rev_per_sec` revolutions per second.
	pub fn from_rev_per_sec(rev_per_sec: f32) -> Self { Self(rev_per_sec * 360.0) }

	/// This rate in degrees per second.
	pub fn as_deg_per_sec(self) -> f32 { self.0 }
	/// This rate in radians per second.
	pub fn as_rad_per_sec(self) -> f32 { self.0 * PI / 180.0 }
	/// This rate in revolutions per minute.
	pub fn as_rpm(self) -> f32 { self.0 / 6.0 }
	/// This rate in revolutions per second.
	pub fn as_rev_per_sec(self) -> f32 { self.0 / 360.0 }
}

/// A linear distance. Inner unit is **millimeters**.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Reflect)]
pub struct Distance(f32);

impl Distance {
	/// A distance of `mm` millimeters.
	pub fn from_millimeters(mm: f32) -> Self { Self(mm) }
	/// A distance of `cm` centimeters.
	pub fn from_centimeters(cm: f32) -> Self { Self(cm * 10.0) }
	/// A distance of `m` meters.
	pub fn from_meters(m: f32) -> Self { Self(m * 1000.0) }
	/// A distance of `inches` inches.
	pub fn from_inches(inches: f32) -> Self { Self(inches * 25.4) }

	/// This distance in millimeters.
	pub fn as_millimeters(self) -> f32 { self.0 }
	/// This distance in centimeters.
	pub fn as_centimeters(self) -> f32 { self.0 / 10.0 }
	/// This distance in meters.
	pub fn as_meters(self) -> f32 { self.0 / 1000.0 }
	/// This distance in inches.
	pub fn as_inches(self) -> f32 { self.0 / 25.4 }
}

/// A linear velocity. Inner unit is **millimeters per second**.
///
/// A plain value, not a component: a driven body carries its commanded forward speed
/// inside a `DifferentialDrive` component,
/// not as a bare `LinearVelocity`. Markup may write a bare number (`linear=60`,
/// interpreted as mm/s) via the [`From`] impls below.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// serialize as a plain number (mm/s), not a `{ 0: .. }` newtype wrapper, so a
// model authors `linear=60` and the robot wire reads one bare value.
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct LinearVelocity(f32);

impl LinearVelocity {
	/// A speed of `mm_per_sec` millimeters per second.
	pub fn from_mm_per_sec(mm_per_sec: f32) -> Self { Self(mm_per_sec) }
	/// A speed of `cm_per_sec` centimeters per second.
	pub fn from_cm_per_sec(cm_per_sec: f32) -> Self { Self(cm_per_sec * 10.0) }
	/// A speed of `m_per_sec` meters per second.
	pub fn from_m_per_sec(m_per_sec: f32) -> Self { Self(m_per_sec * 1000.0) }

	/// This speed in millimeters per second.
	pub fn as_mm_per_sec(self) -> f32 { self.0 }
	/// This speed in centimeters per second.
	pub fn as_cm_per_sec(self) -> f32 { self.0 / 10.0 }
	/// This speed in meters per second.
	pub fn as_m_per_sec(self) -> f32 { self.0 / 1000.0 }
}

// Arithmetic. Each quantity adds/subtracts its own kind and scales by a scalar,
// so callers can write `pose + delta` or `speed * 0.5` without unwrapping.
macro_rules! impl_quantity_ops {
	($ty:ident) => {
		impl Add for $ty {
			type Output = Self;
			fn add(self, rhs: Self) -> Self { Self(self.0 + rhs.0) }
		}
		impl Sub for $ty {
			type Output = Self;
			fn sub(self, rhs: Self) -> Self { Self(self.0 - rhs.0) }
		}
		impl Neg for $ty {
			type Output = Self;
			fn neg(self) -> Self { Self(-self.0) }
		}
		impl Mul<f32> for $ty {
			type Output = Self;
			fn mul(self, rhs: f32) -> Self { Self(self.0 * rhs) }
		}
	};
}

impl_quantity_ops!(Angle);
impl_quantity_ops!(AngularVelocity);
impl_quantity_ops!(Distance);
impl_quantity_ops!(LinearVelocity);

// A bare markup number (`<SetDrive linear=60 angular=90>`) coerces into the velocity
// through `From`, in the type's natural authoring unit (mm/s for linear, deg/s for
// angular), so a scene reads in real-world units without a constructor call.
macro_rules! impl_velocity_from {
	($ty:ident, $ctor:ident) => {
		impl From<f32> for $ty {
			fn from(value: f32) -> Self { Self::$ctor(value) }
		}
		impl From<f64> for $ty {
			fn from(value: f64) -> Self { Self::$ctor(value as f32) }
		}
		impl From<i32> for $ty {
			fn from(value: i32) -> Self { Self::$ctor(value as f32) }
		}
		impl From<u32> for $ty {
			fn from(value: u32) -> Self { Self::$ctor(value as f32) }
		}
	};
}

impl_velocity_from!(LinearVelocity, from_mm_per_sec);
impl_velocity_from!(AngularVelocity, from_deg_per_sec);

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn angle_round_trips() {
		Angle::from_degrees(180.0).as_radians().xpect_close(PI);
		Angle::from_revolutions(1.0).as_degrees().xpect_close(360.0);
		Angle::from_percent(100.0).as_revolutions().xpect_close(1.0);
		Angle::from_degrees(90.0).as_percent().xpect_close(25.0);
	}

	#[crate::test]
	fn distance_round_trips() {
		Distance::from_meters(1.0).as_millimeters().xpect_close(1000.0);
		Distance::from_inches(1.0).as_millimeters().xpect_close(25.4);
		Distance::from_centimeters(50.0).as_meters().xpect_close(0.5);
	}

	#[crate::test]
	fn velocity_and_arithmetic() {
		AngularVelocity::from_rpm(60.0).as_rev_per_sec().xpect_close(1.0);
		LinearVelocity::from_m_per_sec(1.0)
			.as_mm_per_sec()
			.xpect_close(1000.0);
		(Distance::from_meters(1.0) + Distance::from_meters(0.5))
			.as_millimeters()
			.xpect_close(1500.0);
		(Distance::from_millimeters(300.0) * 0.5)
			.as_millimeters()
			.xpect_close(150.0);
	}
}
