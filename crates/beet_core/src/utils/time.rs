//! Cross-platform timer utility for beet_core
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;


// #[deprecated = "use web_time"]
#[derive(Debug, Clone, Copy)]
pub struct CrossInstant {
	#[cfg(target_arch = "wasm32")]
	start: f64,
	#[cfg(not(target_arch = "wasm32"))]
	start: Instant,
}

impl CrossInstant {
	/// Create a new timer CrossInstant
	pub fn now() -> Self {
		CrossInstant {
			#[cfg(target_arch = "wasm32")]
			start: Self::performance_now(),
			#[cfg(not(target_arch = "wasm32"))]
			start: Instant::now(),
		}
	}

	#[cfg(target_arch = "wasm32")]
	pub fn performance_now() -> f64 {
		let perf = web_sys::window()
			.and_then(|win| win.performance())
			.expect("performance API not available");
		perf.now()
	}

	/// Returns elapsed time in seconds since creation
	pub fn elapsed(&self) -> Duration {
		#[cfg(target_arch = "wasm32")]
		{
			let elapsed_secs = (Self::performance_now() - self.start) / 1000.0;
			Duration::from_secs_f64(elapsed_secs)
		}
		#[cfg(not(target_arch = "wasm32"))]
		{
			self.start.elapsed()
		}
	}

	pub fn add(&self, duration: Duration) -> Self {
		CrossInstant {
			#[cfg(target_arch = "wasm32")]
			start: self.start + duration.as_millis() as f64 / 1000.0,
			#[cfg(not(target_arch = "wasm32"))]
			start: self.start + duration,
		}
	}

	/// ## Panics
	///
	/// Panics if the system time is before the Unix epoch.
	pub fn unix_epoch() -> Duration {
		#[cfg(target_arch = "wasm32")]
		{
			let date = js_sys::Date::new_0();
			Duration::from_secs_f64(date.get_time() / 1000.0)
		}
		#[cfg(not(target_arch = "wasm32"))]
		{
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.expect("SystemTime::now is before UNIX_EPOH")
		}
	}
}


// wasm needs async
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let now = CrossInstant::now();
		std::thread::sleep(std::time::Duration::from_millis(100));
		let elapsed = now.elapsed().as_secs_f64();
		(elapsed >= 0.1 && elapsed < 0.2).xpect_true();
	}
}
