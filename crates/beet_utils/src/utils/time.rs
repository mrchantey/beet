//! Cross-platform timer utility for beet_utils
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

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
}


// wasm needs async
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn works() {
		let now = CrossInstant::now();
		std::thread::sleep(std::time::Duration::from_millis(100));
		let elapsed = now.elapsed().as_secs_f64();
		assert!(
			elapsed >= 0.1 && elapsed < 0.2,
			"Elapsed time was {elapsed}"
		);
	}
}
