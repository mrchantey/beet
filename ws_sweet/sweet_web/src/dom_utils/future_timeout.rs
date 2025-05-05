use crate::timeout_reject;
use anyhow::Result;
use js_sys::Array;
use js_sys::Promise;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::JsValue;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::spawn_local;


pub async fn future_timeout<F, Fut, O>(fut: F, duration: Duration) -> Result<O>
where
	F: 'static + FnOnce() -> Fut,
	Fut: Future<Output = O>,
	O: 'static,
{
	let out = Rc::<RefCell<Option<O>>>::default();

	let mut fut = Some(fut);
	let out2 = out.clone();
	let prom = Promise::new(&mut move |resolve, _reject| {
		let fut = fut.take().unwrap_throw();
		let out = out2.clone();
		spawn_local(async move {
			let result = fut().await;
			*out.borrow_mut() = Some(result);
			resolve.call0(&JsValue::NULL).unwrap();
		});
	});
	let timeout = timeout_reject(duration);

	let arr = Array::new();
	arr.push(&prom);
	arr.push(&timeout);

	match JsFuture::from(Promise::race(&arr)).await {
		Ok(_) => Ok(out.take().unwrap()),
		Err(_) => Err(anyhow::anyhow!("Timeout")),
	}
}


#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod test {
	use crate::prelude::*;
	use std::time::Duration;
	use sweet_test::as_sweet::*;

	#[sweet_test::test]
	pub async fn works() {
		let a = future_timeout(
			|| async {
				wait_for_millis(400).await;
				39
			},
			Duration::from_millis(500),
		)
		.await
		.unwrap();
		expect(a).to_be(39);
	}
	#[sweet_test::test]
	pub async fn times_out() {
		let err = future_timeout(
			|| async {
				wait_for_millis(600).await;
			},
			Duration::from_millis(500),
		)
		.await;
		expect(err).to_be_err_str("Timeout");
	}
}
