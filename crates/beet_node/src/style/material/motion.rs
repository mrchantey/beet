#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;




// ── Duration ref tokens ───────────────────────────────────────────────────────

css_variable!(Short1,      Duration);
css_variable!(Short2,      Duration);
css_variable!(Short3,      Duration);
css_variable!(Short4,      Duration);
css_variable!(Medium1,     Duration);
css_variable!(Medium2,     Duration);
css_variable!(Medium3,     Duration);
css_variable!(Medium4,     Duration);
css_variable!(Long1,       Duration);
css_variable!(Long2,       Duration);
css_variable!(Long3,       Duration);
css_variable!(Long4,       Duration);
css_variable!(ExtraLong1,  Duration);
css_variable!(ExtraLong2,  Duration);
css_variable!(ExtraLong3,  Duration);
css_variable!(ExtraLong4,  Duration);

// ── Motion sys tokens ─────────────────────────────────────────────────────────

css_variable!(MotionStandard,            Motion);
css_variable!(MotionStandardAccelerate,  Motion);
css_variable!(MotionStandardDecelerate,  Motion);
css_variable!(MotionEmphasized,          Motion);
css_variable!(MotionEmphasizedAccelerate,Motion);
css_variable!(MotionEmphasizedDecelerate,Motion);



pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
    .insert(MotionProps)
		.insert(Short1)
		.insert(Short2)
		.insert(Short3)
		.insert(Short4)
		.insert(Medium1)
		.insert(Medium2)
		.insert(Medium3)
		.insert(Medium4)
		.insert(Long1)
		.insert(Long2)
		.insert(Long3)
		.insert(Long4)
		.insert(ExtraLong1)
		.insert(ExtraLong2)
		.insert(ExtraLong3)
		.insert(ExtraLong4)
		.insert(MotionStandard)
		.insert(MotionStandardAccelerate)
		.insert(MotionStandardDecelerate)
		.insert(MotionEmphasized)
		.insert(MotionEmphasizedAccelerate)
		.insert(MotionEmphasizedDecelerate)
}


/// Returns a [`Rule`] with all MD3 duration default values.
pub fn default_durations() -> Rule {
	Rule::new()
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

/// Returns a [`Rule`] with all MD3 motion default values.
///
/// Each [`Motion`] references a duration token via [`FieldRef`] rather than
/// embedding the duration directly.
pub fn default_motions() -> Rule {
	Rule::new()
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
