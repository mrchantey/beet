//! Stream-based requestAnimationFrame (RAF) helper.
//!
//! This provides a `Stream<Item = f64>` of RAF timestamps. It starts immediately
//! on construction and keeps scheduling the next frame until dropped. Dropping
//! cancels the next scheduled frame via `cancelAnimationFrame`.
//!
//! Design
//! - Starts immediately.
//! - Uses a channel to deliver timestamps from the browser callback to async consumers.
//! - Implements `Stream`, so you can `.next().await`.
//! - Cancels automatically on `Drop`. Call `forget()` to leak and keep it running
//!   for the lifetime of the page.
//!
//! Example (requires wasm/DOM):
//! ```ignore
//! use beet_core::web_utils::animation_frame::AnimationFrame;
//! use futures_lite::StreamExt;
//!
//! let mut raf = AnimationFrame::new();
//! let first = raf.next().await.unwrap();
//! let second = raf.next().await.unwrap();
//! assert!(second > first);
//! ```

use async_channel::Receiver;
use async_channel::TryRecvError;
use async_channel::unbounded;
use futures_lite::Stream;

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::window;

/// Inner that owns the RAF closure and the active handle.
///
/// When dropped, it cancels the pending animation frame (if any).
struct AnimationFrameInner {
	handle: Rc<RefCell<i32>>,
	_closure: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
}

impl Drop for AnimationFrameInner {
	fn drop(&mut self) {
		// Cancel the most recent scheduled frame. Ignore errors if it's already gone.
		let id = *self.handle.borrow();
		// 0 is a valid id only after first schedule; if new() failed earlier we wouldn't be here.
		let _ = window().unwrap().cancel_animation_frame(id);
	}
}

/// Stream of requestAnimationFrame timestamps (DOMHighResTimeStamp in milliseconds).
///
/// Use `.next().await` to wait for a single frame or iterate to process many, or
/// call the convenience `next_frame()` method if you don't want to pull in `StreamExt`.
pub struct AnimationFrame {
	receiver: Receiver<f64>,
	// Keep scheduling alive and ensure cleanup on drop.
	_inner: Rc<AnimationFrameInner>,
}

impl Unpin for AnimationFrame {}

impl AnimationFrame {
	/// Start a continuous RAF loop as a `Stream` of timestamps.
	pub fn new() -> Self {
		let (sender, receiver) = unbounded::<f64>();

		let handle: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
		let handle_for_closure = handle.clone();

		// We store the closure in an Rc<RefCell<Option<...>>> so the closure can
		// re-borrow itself by reference when re-scheduling.
		let closure_cell: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> =
			Rc::new(RefCell::new(None));
		let closure_cell_for_init = closure_cell.clone();
		let closure_cell_for_closure = closure_cell.clone();

		// Build the RAF callback. It forwards the timestamp and immediately schedules the next frame.
		*closure_cell_for_init.borrow_mut() =
			Some(Closure::wrap(Box::new(move |timestamp_ms: f64| {
				// Deliver timestamp; ignore if receiver was dropped.
				let _ = sender.try_send(timestamp_ms);

				// Schedule the next frame with the same closure.
				let borrow = closure_cell_for_closure.borrow();
				let closure_ref = borrow.as_ref().unwrap();
				*handle_for_closure.borrow_mut() =
					request_animation_frame(closure_ref);
			}) as Box<dyn FnMut(f64)>));

		// Kick off the first frame.
		{
			let closure_ref = closure_cell.borrow();
			let closure_ref = closure_ref.as_ref().unwrap();
			*handle.borrow_mut() = request_animation_frame(closure_ref);
		}

		let inner = Rc::new(AnimationFrameInner {
			handle,
			_closure: closure_cell,
		});

		Self {
			receiver,
			_inner: inner,
		}
	}

	/// Leak the RAF loop (do not cancel on drop).
	pub fn forget(self) { std::mem::forget(self); }
}

impl Stream for AnimationFrame {
	type Item = f64;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// Fast path: try to grab without parking.
		match this.receiver.try_recv() {
			Ok(ts) => return Poll::Ready(Some(ts)),
			Err(TryRecvError::Closed) => return Poll::Ready(None),
			Err(TryRecvError::Empty) => {}
		}

		// Fallback: poll the async receive future.
		let recv = this.receiver.clone();
		let fut = recv.recv();
		futures_lite::pin!(fut);
		match fut.poll(cx) {
			Poll::Ready(Ok(ts)) => Poll::Ready(Some(ts)),
			Poll::Ready(Err(_closed)) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) -> i32 {
	window()
		.unwrap()
		.request_animation_frame(f.as_ref().unchecked_ref())
		.unwrap()
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
	use super::AnimationFrame;
	use crate::prelude::*;

	#[ignore = "requires dom"]
	#[test]
	fn works() {
		// Ensure minimal DOM access available
		let _ = document_ext::document();
		let _ = document_ext::head();
		let _ = document_ext::body();
	}

	#[ignore = "requires dom"]
	#[sweet::test]
	async fn yields_timestamps() {
		// Ensure clean DOM (not strictly required for RAF, but keeps parity with other tests)
		document_ext::clear_body();

		let mut raf = AnimationFrame::new();

		let a = raf.next().await.unwrap();
		let b = raf.next().await.unwrap();

		(b > a).xpect_true();
	}
}
