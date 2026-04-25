use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// A motion token combining an easing function with a duration token.
#[derive(Debug, Clone, PartialEq, Reflect, FromTokens)]
pub struct Motion {
	/// [`FieldRef`] pointing to the [`Duration`] token.
	#[token]
	pub duration: Duration,
	pub ease: EaseFunction,
}

impl CssValue for Motion {
	fn to_css_value(&self) -> String {
		format!(
			"{} {}",
			self.duration.to_css_value(),
			self.ease.to_css_value()
		)
	}
}

impl CssValue for Duration {
	fn to_css_value(&self) -> String { format!("{}ms", self.as_millis()) }
}

impl CssValue for EaseFunction {
	#[rustfmt::skip]
	fn to_css_value(&self) -> String {
		match self {
			// --- Standard Cubic Beziers ---
			EaseFunction::Linear => 				"linear".to_string(),
			EaseFunction::QuadraticIn => 		"cubic-bezier(0.11, 0, 0.5, 0)".to_string(),
			EaseFunction::QuadraticOut => 	"cubic-bezier(0.5, 1, 0.89, 1)".to_string(),
			EaseFunction::QuadraticInOut => "cubic-bezier(0.45, 0, 0.55, 1)".to_string(),
			EaseFunction::CubicIn => 				"cubic-bezier(0.32, 0, 0.67, 0)".to_string(),
			EaseFunction::CubicOut => 			"cubic-bezier(0.33, 1, 0.68, 1)".to_string(),
			EaseFunction::CubicInOut => 		"cubic-bezier(0.65, 0, 0.35, 1)".to_string(),
			EaseFunction::SineIn => 				"cubic-bezier(0.12, 0, 0.39, 0)".to_string(),
			EaseFunction::SineOut => 				"cubic-bezier(0.61, 1, 0.88, 1)".to_string(),
			EaseFunction::SineInOut => 			"cubic-bezier(0.37, 0, 0.63, 1)".to_string(),
			EaseFunction::BackIn => 				"cubic-bezier(0.36, 0, 0.66, -0.56)".to_string(),
			EaseFunction::BackOut => 				"cubic-bezier(0.34, 1.56, 0.64, 1)".to_string(),
			EaseFunction::BackInOut => 			"cubic-bezier(0.68, -0.6, 0.32, 1.6)".to_string(),
			// --- Steps ---
			EaseFunction::Steps(n, _) => 		format!("steps({}, end)", n),
			// --- Complex approximations using linear() ---
			// SmootherStep (Ken Perlin's 6t^5 - 15t^4 + 10t^3)
			EaseFunction::SmootherStep
			| EaseFunction::SmootherStepIn
			| EaseFunction::SmootherStepOut => 	"linear(0, 0.002, 0.015, 0.05, 0.11, 0.21, 0.35, 0.5, 0.65, 0.79, 0.89, 0.95, 0.985, 0.998, 1)".to_string(),

			// Bounce Out: approximating the "hitting the floor" look
			EaseFunction::BounceOut => 					"linear(0, 0.36, 0.73, 1, 0.81, 1, 0.94, 1, 0.98, 1)".to_string(),
			// Elastic Out: approximating the springy overshoot
			EaseFunction::ElasticOut => 				"linear(0, 1.32, 0.87, 1.05, 0.98, 1.01, 1)".to_string(),
			// Bounce/Elastic In/InOut reverse or extend the above
			EaseFunction::BounceIn => 					"linear(0, 0.02, 0, 0.06, 0, 0.19, 0, 0.27, 0.64, 1)".to_string(),
			EaseFunction::ElasticIn => 					"linear(0, -0.01, 0.02, -0.05, 0.13, -0.32, 1)".to_string(),
			// Fallback for types not explicitly mapped
			_ => 																"ease-in-out".to_string(),
		}
	}
}
