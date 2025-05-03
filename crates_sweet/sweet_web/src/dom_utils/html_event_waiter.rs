use super::*;
use js_sys::Array;
use js_sys::Promise;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use web_sys::EventTarget;


pub struct HtmlEventWaiterInstance {
	pub listener: Rc<RefCell<Option<HtmlEventListener<JsValue>>>>,
	pub promise: Promise,
}

pub struct HtmlEventWaiter {
	name: &'static str,
	target: EventTarget,
	rejects: bool,
}

impl HtmlEventWaiter {
	pub fn new(name: &'static str) -> Self {
		Self::new_with_target(name, window().unwrap())
	}
	pub fn new_with_target(
		name: &'static str,
		target: impl Into<EventTarget>,
	) -> Self {
		Self {
			name,
			target: target.into(),
			rejects: false,
		}
	}
	pub fn rejects(mut self) -> Self {
		self.rejects = true;
		self
	}
	pub fn instantiate(self) -> HtmlEventWaiterInstance {
		let listener: Rc<RefCell<Option<HtmlEventListener<JsValue>>>> =
			Rc::new(RefCell::new(None));

		let listener2 = listener.clone();
		let promise = Promise::new(&mut move |resolve, reject| {
			let func = if self.rejects { reject } else { resolve };
			let target = self.target.clone();
			*listener2.borrow_mut() =
				Some(HtmlEventListener::<JsValue>::new_with_target(
					self.name,
					move |value| {
						func.call1(&JsValue::NULL, &value).unwrap();
					},
					target,
				));
		});
		HtmlEventWaiterInstance { listener, promise }
	}

	pub async fn wait(self) -> Result<JsValue, JsValue> {
		// HtmlEventListener::wait_with_target(self.name, self.target).await
		let inst = self.instantiate();
		JsFuture::from(inst.promise).await
	}
	pub async fn wait_or_timeout(
		self,
		duration: Duration,
	) -> Result<JsValue, JsValue> {
		let inst = self.instantiate();
		let timeout = timeout_reject(duration);

		let arr = Array::new();
		arr.push(&inst.promise);
		arr.push(&timeout);

		JsFuture::from(Promise::race(&arr)).await
	}

	pub async fn await_first(
		items: impl IntoIterator<Item = HtmlEventWaiter>,
	) -> Result<JsValue, JsValue> {
		let items = items
			.into_iter()
			.map(|item| item.instantiate())
			.collect::<Vec<_>>();

		let arr = Array::new();
		for item in items.iter() {
			arr.push(&item.promise);
		}
		JsFuture::from(Promise::race(&arr)).await
	}
}

/// Rejects a promise after a given duration
pub fn timeout_reject(duration: Duration) -> Promise {
	Promise::new(&mut |_resolve, reject| {
		web_sys::window()
			.unwrap()
			.set_timeout_with_callback_and_timeout_and_arguments_1(
				&reject,
				duration.as_millis() as i32,
				&JsValue::from_str("Timed out"),
			)
			.unwrap();
	})
}



#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod test {
	use crate::prelude::*;
	use sweet_test::as_sweet::*;

	#[sweet_test::test]
	#[ignore = "requires dom"]
	async fn html_event_waiter() {
		HtmlEventWaiter::new("click").wait().await.anyhow().unwrap();
	}
}
