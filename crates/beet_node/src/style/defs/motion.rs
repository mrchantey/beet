#![cfg_attr(rustfmt, rustfmt_skip)]
use beet_core::prelude::*;
use crate::style::*;
use crate::token;

token!(Motion, MOTION_STANDARD, "motion-standard");
token!(Motion, MOTION_STANDARD_ACCELERATE, "motion-standard-accelerate");
token!(Motion, MOTION_STANDARD_DECELERATE, "motion-standard-decelerate");
token!(Motion, MOTION_EMPHASIZED, "motion-emphasized");
token!(Motion, MOTION_EMPHASIZED_ACCELERATE, "motion-emphasized-accelerate");
token!(Motion, MOTION_EMPHASIZED_DECELERATE, "motion-emphasized-decelerate");


pub fn default_motions()->TokenStore{
	TokenStore::new()
		.with(MOTION_STANDARD,Motion{
			ease: EaseFunction::CubicInOut,
      duration: Duration::from_millis(300),
		})
		.with(MOTION_STANDARD_ACCELERATE,Motion{
			ease: EaseFunction::CubicIn,
      duration: Duration::from_millis(200),
		})
		.with(MOTION_STANDARD_DECELERATE,Motion{
			ease: EaseFunction::CubicOut,
      duration: Duration::from_millis(250),
		})
		.with(MOTION_EMPHASIZED,Motion{
			ease: EaseFunction::QuinticInOut,
      duration: Duration::from_millis(500),
		})
		.with(MOTION_EMPHASIZED_ACCELERATE,Motion{
			ease: EaseFunction::QuarticIn,
      duration: Duration::from_millis(200),
		})
		.with(MOTION_EMPHASIZED_DECELERATE,Motion{
			ease: EaseFunction::QuarticOut,
      duration: Duration::from_millis(400),
		})
}
