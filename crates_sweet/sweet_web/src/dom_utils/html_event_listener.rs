use crate::*;
use js_sys::Function;
use js_sys::Promise;
use std::cell::RefCell;
use std::rc::Rc;
use sweet_utils::prelude::*;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use web_sys::Event;
use web_sys::EventTarget;

pub struct HtmlEventListenerInner<T> {
	closure: Closure<dyn FnMut(T)>,
	target: EventTarget,
	name: &'static str,
}

impl<T> Drop for HtmlEventListenerInner<T> {
	fn drop(&mut self) {
		let closure: &Function = self.closure.as_ref().unchecked_ref();
		self.target
			.remove_event_listener_with_callback(self.name, closure)
			.anyhow()
			.ok_or(|e| web_sys::console::error_1(&format!("{:?}", e).into()));
	}
}

/// Event listener that unsubscribes on drop.
/// It stores an `Rc<Inner>` so can be safely cloned.
#[derive(Clone)]
pub struct HtmlEventListener<T = Event>(pub Rc<HtmlEventListenerInner<T>>);

impl<T> HtmlEventListener<T> {
	#[must_use]
	pub fn new<F>(name: &'static str, f: F) -> Self
	where
		F: FnMut(T) + 'static,
		T: FromWasmAbi + 'static,
	{
		Self::new_with_target(name, f, window().unwrap())
	}
	#[must_use]
	pub fn new_with_target<F>(
		name: &'static str,
		f: F,
		target: impl Into<EventTarget>,
	) -> Self
	where
		F: FnMut(T) + 'static,
		T: FromWasmAbi + 'static,
	{
		let closure = Closure::from_func(f);
		let target = target.into();
		// let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(_)>);
		target
			.add_event_listener_with_callback(
				name,
				closure.as_ref().unchecked_ref(),
			)
			.unwrap();
		Self(Rc::new(HtmlEventListenerInner {
			target,
			name,
			closure,
		}))
	}
	pub fn forget(self) { std::mem::forget(self); }
}


impl HtmlEventListener<JsValue> {
	pub async fn wait(name: &'static str) -> JsValue {
		Self::wait_with_target(name, window().unwrap().unchecked_into()).await
	}
	pub async fn wait_with_target(
		name: &'static str,
		target: EventTarget,
	) -> JsValue {
		let listener: Rc<RefCell<Option<HtmlEventListener<JsValue>>>> =
			Rc::new(RefCell::new(None));

		let listener2 = listener.clone();
		let promise = Promise::new(&mut move |resolve, _reject| {
			let target = target.clone();
			*listener2.borrow_mut() =
				Some(HtmlEventListener::<JsValue>::new_with_target(
					name,
					move |value| {
						resolve.call1(&JsValue::NULL, &value).unwrap();
					},
					target,
				));
		});
		JsFuture::from(promise).await.unwrap()
	}
	// pub async fn wait_with_target_and_while_listening(
	// 	name: &'static str,
	// 	target: EventTarget,
	// 	mut while_listening: impl FnMut() + 'static,
	// ) -> JsValue {
	// 	let listener: RcCell<Option<HtmlEventListener<JsValue>>> = rccell(None);

	// 	let listener2 = listener.clone();
	// 	let promise = Promise::new(&mut move |resolve, _reject| {
	// 		let target = target.clone();
	// 		*listener2.borrow_mut() =
	// 			Some(HtmlEventListener::<JsValue>::new_with_target(
	// 				name,
	// 				move |value| {
	// 					resolve.call1(&JsValue::NULL, &value).unwrap();
	// 				},
	// 				target,
	// 			));
	// 		while_listening();
	// 	});
	// 	let result = JsFuture::from(promise).await.unwrap();
	// 	drop(listener);
	// 	result
	// }
}
