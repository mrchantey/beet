use crate::prelude::*;
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



/// Runs a Send+Sync function with a timeout on native platforms.
/// Returns `Ok(PanicResult)` if completed, `Err(elapsed)` if timed out.
///
/// On native, spawns the function in a thread and uses `recv_timeout`.
/// On WASM, cannot enforce hard timeouts for sync code, so this is not available.
#[cfg(not(target_arch = "wasm32"))]
pub fn timeout_sync(
	func: impl 'static + Send + Sync + FnOnce() -> Result<(), String>,
	timeout: Duration,
) -> Result<PanicResult, Duration> {
	use std::sync::mpsc;

	let (sender, receiver) = mpsc::channel();
	let timeout_start = Instant::now();

	std::thread::spawn(move || {
		let _ = sender.send(PanicContext::catch(func));
	});

	match receiver.recv_timeout(timeout) {
		Ok(result) => Ok(result),
		Err(mpsc::RecvTimeoutError::Timeout) => Err(timeout_start.elapsed()),
		Err(mpsc::RecvTimeoutError::Disconnected) => {
			Ok(PanicResult::Err("Thread disconnected unexpectedly".into()))
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn works() {
		let now = Instant::now();
		time_ext::sleep(Duration::from_millis(100)).await;
		now.elapsed().as_millis().xpect_greater_or_equal_to(100);
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn timeout_sync_completes() {
		time_ext::timeout_sync(|| Ok(()), Duration::from_millis(100))
			.unwrap()
			.xpect_eq(PanicResult::Ok);
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn timeout_sync_times_out() {
		time_ext::timeout_sync(
			|| {
				std::thread::sleep(Duration::from_millis(200));
				Ok(())
			},
			Duration::from_millis(10),
		)
		.unwrap_err();
	}
}
