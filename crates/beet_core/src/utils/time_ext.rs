use std::time::Duration;
use std::time::SystemTime;


pub fn pretty_print_duration(dur: Duration) -> String {
	let total_secs = dur.as_secs();
	let minutes = total_secs / 60;
	let secs = total_secs % 60;
	let millis = dur.subsec_millis();
	if minutes > 0 {
		format!("{}:{:02}.{:03} m", minutes, secs, millis)
	} else if secs > 0 {
		format!("{}.{:02} s", secs, millis)
	} else if millis > 0 {
		format!("{} ms", millis)
	} else {
		let micros = dur.subsec_micros();
		if micros > 0 {
			format!("{} µs", micros)
		} else {
			let nanos = dur.subsec_nanos();
			format!("{} ns", nanos)
		}
	}
}

pub fn now_millis() -> u128 {
	SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_millis()
}

pub async fn sleep_secs(secs: u64) { sleep(Duration::from_secs(secs)).await; }

pub async fn sleep_millis(millis: u64) {
	sleep(Duration::from_millis(millis)).await;
}

pub async fn sleep_micros(micros: u64) {
	sleep(Duration::from_micros(micros)).await;
}

/// Cross platform sleep function
#[allow(unused)]
pub async fn sleep(duration: Duration) {
	#[cfg(not(target_arch = "wasm32"))]
	{
		async_io::Timer::after(duration).await;
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
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	async fn works() {
		let now = Instant::now();
		time_ext::sleep(Duration::from_millis(100)).await;
		now.elapsed().as_millis().xpect_greater_or_equal_to(100);
	}
}
