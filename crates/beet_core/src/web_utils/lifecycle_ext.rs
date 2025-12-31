use crate::prelude::*;
use js_sys::Array;
use js_sys::Promise;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::JsValue;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Leak a value for the lifetime of the page by moving it into a JS Closure
/// and forgetting the closure. Useful for keeping resources or state alive
/// across the entire wasm session (e.g., long-lived event handlers).
pub fn forget<T>(val: T) {
	let closure = Closure::<dyn FnMut()>::new(move || {
		let _ = val;
	});
	closure.forget();
}
/// Leak a `FnMut()` callback for the lifetime of the page by wrapping it in a
/// JS Closure and forgetting it. This is commonly used to install long-lived
/// DOM event handlers in wasm.
pub fn forget_func<F>(f: F)
where
	F: FnMut() + 'static,
{
	let closure = Closure::<dyn FnMut()>::new(f);
	closure.forget();
}



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
		async_ext::spawn_local(async move {
			let result = fut().await;
			*out.borrow_mut() = Some(result);
			resolve.call0(&JsValue::NULL).unwrap();
		}).detach();
	});
	let timeout = timeout_reject(duration);

	let arr = Array::new();
	arr.push(&prom);
	arr.push(&timeout);

	match JsFuture::from(Promise::race(&arr)).await {
		Ok(_) => Ok(out.take().unwrap()),
		Err(_) => Err(bevyhow!("Timeout")),
	}
}


fn timeout_reject(duration: Duration) -> Promise {
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
	use std::time::Duration;
	use sweet::prelude::*;

	#[sweet::test]
	pub async fn works() {
		lifecycle_ext::future_timeout(
			|| async {
				time_ext::sleep(Duration::from_millis(400)).await;
				39
			},
			Duration::from_millis(500),
		)
		.await
		.unwrap()
		.xpect_eq(39);
	}
	#[sweet::test]
	pub async fn times_out() {
		lifecycle_ext::future_timeout(
			|| async {
				time_ext::sleep(Duration::from_millis(600)).await;
			},
			Duration::from_millis(500),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_eq("Timeout\n");
	}
}
