#![allow(clippy::needless_doctest_main)]
//! Stream-based DOM event listener.
//!
//! This replaces ad-hoc "wait once" helpers with a proper `Stream` you can
//! `.next().await` or iterate to consume all events.
//!
//! Design
//! - Unsubscribes automatically on `Drop`.
//! - Uses a channel to forward events from the browser to Rust async tasks.
//! - `new` listens on `window` by default; use `new_with_target` to listen on a specific element.
//!
//! Examples
//! Create a stream of `click` events and await the next one:
//! ```ignore
//! use beet_core::web_utils::html_event_listener::HtmlEventListener;
//! use futures_lite::StreamExt;
//!
//! // Listen on the window for MouseEvent clicks
//! let mut clicks = HtmlEventListener::<web_sys::MouseEvent>::new("click");
//! let first = clicks.next().await.unwrap();
//! assert_eq!(first.type_(), "click");
//! ```
//!
//! Attach to a specific element:
//! ```ignore
//! use beet_core::web_utils::{document_ext as doc, html_event_listener::HtmlEventListener};
//! use futures_lite::StreamExt;
//! use web_sys::HtmlButtonElement;
//!
//! let button: HtmlButtonElement = doc::create_button();
//! button.set_inner_html("Press me");
//! doc::append_child(&button);
//!
//! let mut stream = HtmlEventListener::<web_sys::MouseEvent>::new_with_target(
//!     "click",
//!     button.clone(),
//! );
//!
//! button.click(); // trigger programmatically
//! let ev = stream.next().await.unwrap();
//! assert_eq!(ev.type_(), "click");
//! ```


use async_channel::Receiver;
use async_channel::Sender;
use async_channel::TryRecvError;
use async_channel::unbounded;
use futures_lite::Stream;
use js_sys::Function;

use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;
use wasm_bindgen::JsCast;

use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::Closure;

use web_sys::EventTarget;
use web_sys::Window;

/// Inner listener that owns the JS closure and unsubscribes on drop.
struct HtmlEventListenerInner<T> {
	pub name: &'static str,
	pub target: EventTarget,
	pub closure: Closure<dyn FnMut(T)>,
}

impl<T> Drop for HtmlEventListenerInner<T> {
	fn drop(&mut self) {
		let closure: &Function = self.closure.as_ref().unchecked_ref();
		let _ = self
			.target
			.remove_event_listener_with_callback(self.name, closure);
	}
}

/// Stream of DOM events. Unsubscribes on drop.
///
/// Use `.next().await` to wait for a single event or iterate to process multiple.
pub struct HtmlEventListener<T = web_sys::Event> {
	receiver: Receiver<T>,
	// Keep the listener alive and ensure cleanup on drop.
	_inner: Rc<HtmlEventListenerInner<T>>,
}
impl<T> Unpin for HtmlEventListener<T> {}

impl<T> HtmlEventListener<T>
where
	T: 'static + FromWasmAbi,
{
	/// Listen for `name` events on the global `window`.
	pub fn new(name: &'static str) -> Self {
		let window: Window = web_sys::window().unwrap();
		Self::new_with_target(name, window)
	}

	/// Listen for `name` events on `target`.
	pub fn new_with_target(
		name: &'static str,
		target: impl Into<EventTarget>,
	) -> Self {
		let (sender, receiver): (Sender<T>, Receiver<T>) = unbounded();
		let target = target.into();

		let closure = Closure::wrap(Box::new(move |value: T| {
			// Ignore send errors if receiver was dropped.
			let _ = sender.try_send(value);
		}) as Box<dyn FnMut(T)>);

		target
			.add_event_listener_with_callback(
				name,
				closure.as_ref().unchecked_ref(),
			)
			.unwrap();

		let inner = Rc::new(HtmlEventListenerInner {
			name,
			target,
			closure,
		});

		Self {
			receiver,
			_inner: inner,
		}
	}

	/// Leak the listener (do not unsubscribe). Useful for long-lived global listeners.
	pub fn forget(self) { std::mem::forget(self); }

	/// Await the next event from this listener.
	/// Convenience when you don't want to depend on StreamExt::next.
	pub async fn next_event(&mut self) -> Option<T> {
		self.receiver.recv().await.ok()
	}
}

impl<T> Stream for HtmlEventListener<T> {
	type Item = T;

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

		// No item immediately available; poll using a cloned receiver to avoid borrowing self across await.
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
	use super::HtmlEventListener;
	use crate::web_utils::document_ext as doc;

	use crate::prelude::*;
	use web_sys::HtmlButtonElement;
	use web_sys::MouseEvent;

	#[ignore = "requires dom"]
	#[test]
	fn works() {
		// Ensure minimal DOM access available
		let _ = doc::document();
		let _ = doc::head();
		let _ = doc::body();
	}

	#[ignore = "requires dom"]
	#[crate::test]
	async fn works_async() {
		doc::clear_body();

		let button: HtmlButtonElement = doc::create_button();
		button.set_id("clicker");
		button.set_inner_html("Click me");
		doc::append_child(&button);

		let mut clicks = HtmlEventListener::<MouseEvent>::new_with_target(
			"click",
			button.clone(),
		);

		// Trigger programmatically
		button.click();

		let ev = clicks.next_event().await.unwrap();
		ev.type_().xpect_eq("click");
	}
}
