use js_sys::Array;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::Element;
use web_sys::HtmlCanvasElement;
use web_sys::ResizeObserver;
use web_sys::ResizeObserverEntry;
use web_sys::ResizeObserverSize;


/// Resize listener
/// When using with leptos, ensure it is moved to [`on_cleanup`] to avoid being dropped
pub struct ResizeListener {
	pub observer: ResizeObserver,
	pub cb: Closure<dyn FnMut(Array, ResizeObserver)>,
}

impl ResizeListener {
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

	pub fn forget(self) { std::mem::forget(self); }
	/// utility function for parsing the entry, usually this is what you want
	/// https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserverEntry/contentBoxSize
	pub fn parse_entry(entry: &ResizeObserverEntry) -> (u32, u32) {
		let first = entry.content_box_size().get(0);
		let first = first.unchecked_ref::<ResizeObserverSize>();
		let width = first.inline_size();
		let height = first.block_size();
		(width as u32, height as u32)
	}
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