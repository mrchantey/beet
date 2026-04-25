#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

// ── Duration ref tokens ───────────────────────────────────────────────────────

token2!(Short1,      Duration);
token2!(Short2,      Duration);
token2!(Short3,      Duration);
token2!(Short4,      Duration);
token2!(Medium1,     Duration);
token2!(Medium2,     Duration);
token2!(Medium3,     Duration);
token2!(Medium4,     Duration);
token2!(Long1,       Duration);
token2!(Long2,       Duration);
token2!(Long3,       Duration);
token2!(Long4,       Duration);
token2!(ExtraLong1,  Duration);
token2!(ExtraLong2,  Duration);
token2!(ExtraLong3,  Duration);
token2!(ExtraLong4,  Duration);

// ── Motion sys tokens ─────────────────────────────────────────────────────────

token2!(MotionStandard,            Motion);
token2!(MotionStandardAccelerate,  Motion);
token2!(MotionStandardDecelerate,  Motion);
token2!(MotionEmphasized,          Motion);
token2!(MotionEmphasizedAccelerate,Motion);
token2!(MotionEmphasizedDecelerate,Motion);

/// Returns a [`Selector`] with all MD3 duration default values.
pub fn default_durations() -> Selector {
	Selector::new()
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
	Selector::new()
		.with_value::<MotionStandard>(Motion {
			ease:     EaseFunction::CubicInOut,
			duration: FieldRef::of::<Medium2>(),
		}).unwrap()
		.with_value::<MotionStandardAccelerate>(Motion {
			ease:     EaseFunction::CubicIn,
			duration: FieldRef::of::<Short4>(),
		}).unwrap()
		.with_value::<MotionStandardDecelerate>(Motion {
			ease:     EaseFunction::CubicOut,
			duration: FieldRef::of::<Medium1>(),
		}).unwrap()
		.with_value::<MotionEmphasized>(Motion {
			ease:     EaseFunction::QuinticInOut,
			duration: FieldRef::of::<Long2>(),
		}).unwrap()
		.with_value::<MotionEmphasizedAccelerate>(Motion {
			ease:     EaseFunction::QuarticIn,
			duration: FieldRef::of::<Short4>(),
		}).unwrap()
		.with_value::<MotionEmphasizedDecelerate>(Motion {
			ease:     EaseFunction::QuarticOut,
			duration: FieldRef::of::<Medium4>(),
		}).unwrap()
}
