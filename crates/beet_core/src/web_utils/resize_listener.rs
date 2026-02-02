//! Resize observer utilities for monitoring element dimensions.
//!
//! This module provides a wrapper around the browser's [`ResizeObserver`] API
//! for detecting when elements change size.

use js_sys::Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::Element;
use web_sys::HtmlCanvasElement;
use web_sys::ResizeObserver;
use web_sys::ResizeObserverEntry;
use web_sys::ResizeObserverSize;


/// Wrapper around [`ResizeObserver`] that monitors element size changes,
/// removed on drop.
pub struct ResizeListener {
	/// The underlying resize observer.
	pub observer: ResizeObserver,
	/// The callback closure invoked on resize events.
	pub cb: Closure<dyn FnMut(Array, ResizeObserver)>,
}

impl ResizeListener {
	/// Creates a new resize listener that calls `f` when the element resizes.
	///
	/// The callback receives a [`ResizeObserverEntry`] containing size information.
	pub fn new<F>(el: &Element, mut f: F) -> Self
	where
		F: FnMut(&ResizeObserverEntry) + 'static,
	{
		let cb = Closure::wrap(Box::new(
			move |entries: Array, _observer: ResizeObserver| {
				let entry = entries.get(0);
				let entry = entry.dyn_ref::<ResizeObserverEntry>().unwrap();
				f(entry);
			},
		) as Box<dyn FnMut(Array, ResizeObserver)>);
		let observer =
			ResizeObserver::new(&cb.as_ref().unchecked_ref()).unwrap();
		observer.observe(el);
		Self { cb, observer }
	}

	/// Leaks the listener to keep it alive for the page lifetime.
	pub fn forget(self) { std::mem::forget(self); }
	/// Parses a resize entry into `(width, height)` dimensions.
	///
	/// Uses the `contentBoxSize` which is typically what you want.
	/// See [MDN docs](https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserverEntry/contentBoxSize).
	pub fn parse_entry(entry: &ResizeObserverEntry) -> (u32, u32) {
		let first = entry.content_box_size().get(0);
		let first = first.unchecked_ref::<ResizeObserverSize>();
		let width = first.inline_size();
		let height = first.block_size();
		(width as u32, height as u32)
	}
	/// Creates a listener that automatically resizes a canvas to match its container.
	pub fn resize_canvas(canvas: HtmlCanvasElement) -> Self {
		let canvas2 = canvas.clone();
		Self::new(canvas.unchecked_ref(), move |entry| {
			let content_rect = entry.content_rect();
			canvas2.set_width(content_rect.width() as u32);
			canvas2.set_height(content_rect.height() as u32);
		})
	}
}

impl Drop for ResizeListener {
	fn drop(&mut self) { self.observer.disconnect(); }
}
