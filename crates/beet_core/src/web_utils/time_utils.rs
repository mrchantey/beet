//! High-resolution timing utilities for WebAssembly environments.
//!
//! Provides access to the browser's `Performance.now()` API for
//! sub-millisecond timing measurements.

use web_sys::window;

/// Returns the current high-resolution timestamp in milliseconds.
///
/// This wraps the browser's [`Performance.now()`](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now)
/// API, providing sub-millisecond precision for timing measurements.
///
/// # Panics
///
/// Panics if called outside a browser environment (no `window` object).
pub fn performance_now() -> f64 {
	window().unwrap().performance().unwrap().now()
}
