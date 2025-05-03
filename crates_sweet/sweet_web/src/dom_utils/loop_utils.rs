use js_sys::Promise;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_bindgen_futures::JsFuture;


pub fn block_forever() {
	let _ = future_to_promise(loop_forever_wasm());
	loop {}
}

async fn loop_forever_wasm() -> Result<JsValue, JsValue> {
	loop_forever().await;
}

pub async fn loop_forever() -> ! {
	loop {
		wait_for_16_millis().await;
	}
}

pub async fn wait_for_16_millis() { wait_for_millis(16).await }

pub async fn wait_for_millis(millis: i32) {
	let promise = Promise::new(&mut |resolve, _reject| {
		web_sys::window()
			.unwrap()
			.set_timeout_with_callback_and_timeout_and_arguments_0(
				&resolve, millis,
			)
			.expect("Should register `setTimeout`.");
	});
	JsFuture::from(promise).await.unwrap();
}

pub async fn run_on_timeout<F>(f: F)
where
	F: FnOnce() + 'static,
{
	let promise = Promise::new(&mut |resolve, _reject| {
		web_sys::window()
			.unwrap()
			.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
			.expect("Should register `setTimeout`.");
	});
	JsFuture::from(promise).await.unwrap();
	f();
}
