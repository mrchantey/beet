use std::time::Duration;


pub async fn sleep_secs(secs: u64) { sleep(Duration::from_secs(secs)).await; }

pub async fn sleep_millis(millis: u64) {
	sleep(Duration::from_millis(millis)).await;
}

/// Cross platform sleep function
#[allow(unused)]
pub async fn sleep(duration: Duration) {
	#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
	{
		tokio::time::sleep(duration).await;
	}
	#[cfg(target_arch = "wasm32")]
	{
		use wasm_bindgen_futures::JsFuture;
		use web_sys::window;
		let window = window().unwrap();
		let promise = js_sys::Promise::new(&mut |resolve, _| {
			window
				.set_timeout_with_callback_and_timeout_and_arguments_0(
					&resolve,
					duration.as_millis() as i32,
				)
				.expect("should register `setTimeout` OK");
		});

		JsFuture::from(promise)
			.await
			.expect("should await `setTimeout` OK");
	}
	#[cfg(not(any(feature = "tokio", target_arch = "wasm32")))]
	panic!("enable tokio feature for sleep on non wasm32 targets");
}
