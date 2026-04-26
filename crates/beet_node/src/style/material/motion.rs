#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

// ── Duration ref tokens ───────────────────────────────────────────────────────

token!(Short1,      Duration);
token!(Short2,      Duration);
token!(Short3,      Duration);
token!(Short4,      Duration);
token!(Medium1,     Duration);
token!(Medium2,     Duration);
token!(Medium3,     Duration);
token!(Medium4,     Duration);
token!(Long1,       Duration);
token!(Long2,       Duration);
token!(Long3,       Duration);
token!(Long4,       Duration);
token!(ExtraLong1,  Duration);
token!(ExtraLong2,  Duration);
token!(ExtraLong3,  Duration);
token!(ExtraLong4,  Duration);

// ── Motion sys tokens ─────────────────────────────────────────────────────────

token!(MotionStandard,            Motion);
token!(MotionStandardAccelerate,  Motion);
token!(MotionStandardDecelerate,  Motion);
token!(MotionEmphasized,          Motion);
token!(MotionEmphasizedAccelerate,Motion);
token!(MotionEmphasizedDecelerate,Motion);

/// Returns a [`Selector`] with all MD3 duration default values.
pub fn default_durations() -> Selector {
	Selector::root()
		.with_value::<Short1>(Duration::from_millis(50)).unwrap()
		.with_value::<Short2>(Duration::from_millis(100)).unwrap()
		.with_value::<Short3>(Duration::from_millis(150)).unwrap()
		.with_value::<Short4>(Duration::from_millis(200)).unwrap()
		.with_value::<Medium1>(Duration::from_millis(250)).unwrap()
		.with_value::<Medium2>(Duration::from_millis(300)).unwrap()
		.with_value::<Medium3>(Duration::from_millis(350)).unwrap()
		.with_value::<Medium4>(Duration::from_millis(400)).unwrap()
		.with_value::<Long1>(Duration::from_millis(450)).unwrap()
		.with_value::<Long2>(Duration::from_millis(500)).unwrap()
		.with_value::<Long3>(Duration::from_millis(550)).unwrap()
		.with_value::<Long4>(Duration::from_millis(600)).unwrap()
		.with_value::<ExtraLong1>(Duration::from_millis(700)).unwrap()
		.with_value::<ExtraLong2>(Duration::from_millis(800)).unwrap()
		.with_value::<ExtraLong3>(Duration::from_millis(900)).unwrap()
		.with_value::<ExtraLong4>(Duration::from_millis(1000)).unwrap()
}

/// Returns a [`Selector`] with all MD3 motion default values.
///
/// Each [`Motion`] references a duration token via [`FieldRef`] rather than
/// embedding the duration directly.
pub fn default_motions() -> Selector {
	Selector::root()
		.with_value::<MotionStandard>(Motion {
			ease:     EaseFunction::CubicInOut,
			duration: Medium2::token(),
		}).unwrap()
		.with_value::<MotionStandardAccelerate>(Motion {
			ease:     EaseFunction::CubicIn,
			duration: Short4::token(),
		}).unwrap()
		.with_value::<MotionStandardDecelerate>(Motion {
			ease:     EaseFunction::CubicOut,
			duration: Medium1::token(),
		}).unwrap()
		.with_value::<MotionEmphasized>(Motion {
			ease:     EaseFunction::QuinticInOut,
			duration: Long2::token(),
		}).unwrap()
		.with_value::<MotionEmphasizedAccelerate>(Motion {
			ease:     EaseFunction::QuarticIn,
			duration: Short4::token(),
		}).unwrap()
		.with_value::<MotionEmphasizedDecelerate>(Motion {
			ease:     EaseFunction::QuarticOut,
			duration: Medium4::token(),
		}).unwrap()
}
