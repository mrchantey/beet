use beet_flow::prelude::RunTimer;
use beet_core::prelude::*;
use std::time::Duration;

/// Sometimes we want a fixed duration and others a
/// consistent speed. This enum allows for both.
#[derive(Debug, Copy, Clone, Reflect)]
pub enum ProceduralAnimationSpeed {
	/// A fixed duration for the animation.
	Duration(Duration),
	/// A fixed speed in meters per second.
	MetersPerSecond(f32),
}


impl Default for ProceduralAnimationSpeed {
	fn default() -> Self { Self::Duration(Duration::from_secs(1)) }
}


impl ProceduralAnimationSpeed {
	/// Calculate the current `t` value for the procedural animation.
	/// - For m/s this will use [`Time::elapsed_secs()`] and `total_len_meters`.
	/// - for `Duration` this will use the [`RunTimer::last_started`]
	pub fn calculate_t(
		&self,
		total_len_meters: f32,
		run_timer: &RunTimer,
	) -> f32 {
		let duration_secs = match self {
			Self::Duration(duration) => duration.as_secs_f32(),
			Self::MetersPerSecond(mps) => total_len_meters / mps,
		};
		run_timer.last_run.elapsed().as_secs_f32() / duration_secs
	}
}
