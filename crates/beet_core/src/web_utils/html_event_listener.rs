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
//! - One listener may [`listen`](HtmlEventListener::listen) for several events
//!   on several targets: every subscription feeds the one queue, so a
//!   frame-driven consumer draining it with
//!   [`try_next_event`](HtmlEventListener::try_next_event) sees events of every kind in
//!   the order they fired.
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
use async_channel::unbounded;
use futures_lite::Stream;
use js_sys::Function;

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use wasm_bindgen::JsCast;

use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::Closure;

use web_sys::EventTarget;
use web_sys::Window;

/// One subscription: owns the JS closure and unsubscribes on drop.
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
	receiver: super::RecvStream<T>,
	/// The queue's send half, cloned into each subscription's closure.
	sender: Sender<T>,
	// Every subscription feeding the queue, kept alive and cleaned up on drop.
	listeners: Vec<HtmlEventListenerInner<T>>,
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
		Self::queue().listen(name, target)
	}

	/// An empty queue with no subscription yet: [`listen`](Self::listen) adds
	/// them, and events from every one drain through it in the order they
	/// fired.
	pub fn queue() -> Self {
		let (sender, receiver): (Sender<T>, Receiver<T>) = unbounded();
		Self {
			receiver: super::RecvStream::new(receiver),
			sender,
			listeners: Vec::new(),
		}
	}

	/// Also deliver `name` events from `target` into this queue.
	pub fn listen(
		self,
		name: &'static str,
		target: impl Into<EventTarget>,
	) -> Self {
		self.listen_filtered(name, target, |_| true)
	}

	/// Also deliver the `name` events from `target` that `filter` accepts.
	///
	/// The filter runs inside the browser's dispatch, the one place a
	/// `preventDefault` counts; a `false` drops the event before it is queued.
	pub fn listen_filtered(
		mut self,
		name: &'static str,
		target: impl Into<EventTarget>,
		mut filter: impl 'static + FnMut(&T) -> bool,
	) -> Self {
		let target = target.into();
		let sender = self.sender.clone();
		let closure = Closure::wrap(Box::new(move |value: T| {
			// Ignore send errors if receiver was dropped.
			if filter(&value) {
				let _ = sender.try_send(value);
			}
		}) as Box<dyn FnMut(T)>);
		target
			.add_event_listener_with_callback(
				name,
				closure.as_ref().unchecked_ref(),
			)
			.unwrap();
		self.listeners.push(HtmlEventListenerInner {
			name,
			target,
			closure,
		});
		self
	}

	/// Leak the listener (do not unsubscribe). Useful for long-lived global listeners.
	pub fn forget(self) { std::mem::forget(self); }

	/// Await the next event from this listener.
	/// Convenience when you don't want to depend on StreamExt::next.
	pub async fn next_event(&mut self) -> Option<T> {
		self.receiver.recv().await
	}

	/// The next event already delivered, without waiting: how a frame-driven
	/// consumer drains the queue.
	pub fn try_next_event(&mut self) -> Option<T> { self.receiver.try_recv() }
}

impl<T: 'static> Stream for HtmlEventListener<T> {
	type Item = T;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		// `RecvStream` holds its recv future across polls, keeping the waker
		// registered for the event that does arrive
		Pin::new(&mut self.get_mut().receiver).poll_next(cx)
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

	#[crate::test(browser)]
	fn works() {
		// Ensure minimal DOM access available
		let _ = doc::document();
		let _ = doc::head();
		let _ = doc::body();
	}

	#[crate::test(browser)]
	async fn works_async() {
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

	/// Several subscriptions feed one queue in firing order, a filter drops
	/// what it refuses, and `try_next_event` drains without waiting.
	#[crate::test(browser)]
	fn a_queue_drains_in_firing_order() {
		let button: HtmlButtonElement = doc::create_button();
		doc::append_child(&button);
		let mut queue = HtmlEventListener::<web_sys::Event>::queue()
			.listen("click", button.clone())
			.listen_filtered("focusin", button.clone(), |ev| {
				ev.prevent_default();
				false
			})
			.listen("custom", button.clone());
		queue.try_next_event().xpect_none();
		button.click();
		button
			.dispatch_event(&web_sys::Event::new("focusin").unwrap())
			.unwrap();
		button
			.dispatch_event(&web_sys::Event::new("custom").unwrap())
			.unwrap();
		queue.try_next_event().unwrap().type_().xpect_eq("click");
		queue.try_next_event().unwrap().type_().xpect_eq("custom");
		queue.try_next_event().xpect_none();
	}
}
