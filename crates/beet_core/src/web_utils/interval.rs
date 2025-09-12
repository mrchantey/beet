#![allow(clippy::needless_doctest_main)]
//! Stream-based `setInterval` helper.
//!
//! This provides a `Stream<Item = f64>` of timestamps (ms, from `performance.now()`).
//! It starts the interval immediately and clears it on `Drop`. Call `forget()`
//! to leak and keep it running for the lifetime of the page.
//!
//! Design
//! - Starts immediately.
//! - Uses a channel to deliver ticks from the browser callback to async consumers.
//! - Implements `Stream`, so you can `.next().await` ticks, or use the provided
//!   `next_tick()` convenience method.
//! - Clears automatically on `Drop`.
//!
//! Example (requires wasm/DOM):
//! ```ignore
//! use beet_core::web_utils::interval::Interval;
//! use futures_lite::StreamExt;
//!
//! let mut every_second = Interval::new(1000);
//! let t1 = every_second.next().await.unwrap();
//! let t2 = every_second.next().await.unwrap();
//! assert!(t2 >= t1);
//! ```
//!
//! If you want frame-aligned updates, see `animation_frame::AnimationFrame`.
use async_channel::Receiver;
use async_channel::TryRecvError;
use async_channel::unbounded;
use futures_lite::Stream;

use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::Window;
use web_sys::window;

/// Inner owner of the JS callback and the interval handle.
/// Clears the interval on drop.
struct IntervalInner {
	target: Window,
	handle: i32,
	_closure: Closure<dyn FnMut()>,
}

impl Drop for IntervalInner {
	fn drop(&mut self) {
		// Clearing an already-cleared handle is a no-op in browsers.
		self.target.clear_interval_with_handle(self.handle);
	}
}

/// Stream of `setInterval` ticks providing `performance.now()` timestamps (milliseconds).
///
/// Use `.next().await` to wait for ticks, or `next_tick()` if you don't want to
/// bring in `StreamExt`.
pub struct Interval {
	receiver: Receiver<f64>,
	// Keep the callback alive and ensure cleanup on drop.
	_inner: Rc<IntervalInner>,
}

impl Unpin for Interval {}

impl Interval {
	/// Start a repeating timer on the global `window` that yields a timestamp every `interval_ms`.
	pub fn new(interval_ms: i32) -> Self {
		let target: Window = window().unwrap();
		Self::new_with_target(interval_ms, target)
	}

	/// Start a repeating timer on `target` that yields a timestamp every `interval_ms`.
	pub fn new_with_target(interval_ms: i32, target: Window) -> Self {
		let (sender, receiver) = unbounded::<f64>();

		let closure = Closure::wrap(Box::new(move || {
			// Prefer high-resolution performance clock. If unavailable, fall back to 0.0.
			let ts = window()
				.and_then(|w| w.performance())
				.map(|p| p.now())
				.unwrap_or(0.0);

			// Ignore send errors if receiver was dropped.
			let _ = sender.try_send(ts);
		}) as Box<dyn FnMut()>);

		let handle = target
			.set_interval_with_callback_and_timeout_and_arguments_0(
				closure.as_ref().unchecked_ref(),
				interval_ms,
			)
			.unwrap();

		let inner = Rc::new(IntervalInner {
			target,
			handle,
			_closure: closure,
		});

		Self {
			receiver,
			_inner: inner,
		}
	}

	/// Leak the interval (do not clear on drop).
	pub fn forget(self) { std::mem::forget(self); }

	/// Await the next interval timestamp.
	/// Convenience when you don't want to depend on StreamExt::next.
	pub async fn next_tick(&mut self) -> Option<f64> {
		self.receiver.recv().await.ok()
	}
}

impl Stream for Interval {
	type Item = f64;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// Try fast-path without registering the waker.
		match this.receiver.try_recv() {
			Ok(item) => return Poll::Ready(Some(item)),
			Err(TryRecvError::Closed) => return Poll::Ready(None),
			Err(TryRecvError::Empty) => {}
		}

		// No item immediately available; poll using a cloned receiver.
		let recv = this.receiver.clone();
		let fut = recv.recv();
		futures_lite::pin!(fut);
		match fut.poll(cx) {
			Poll::Ready(Ok(item)) => Poll::Ready(Some(item)),
			Poll::Ready(Err(_closed)) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
	use super::Interval;
	use crate::web_utils::document_ext as doc;

	use sweet::prelude::*;

	#[ignore = "requires dom"]
	#[test]
	fn works() {
		// Ensure minimal DOM access available
		let _ = doc::document();
		let _ = doc::head();
		let _ = doc::body();
	}

	#[ignore = "requires dom"]
	#[sweet::test]
	async fn yields_timestamps() {
		doc::clear_body();

		let mut interval = Interval::new(10);

		let a = interval.next_tick().await.unwrap();
		let b = interval.next_tick().await.unwrap();

		(b >= a).xpect_true();
	}
}
