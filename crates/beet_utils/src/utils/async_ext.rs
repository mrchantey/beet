use crate::utils::CrossInstant;
use std::time::Duration;

/// Retries a function until it returns Ok or the timeout is reached.
pub async fn retry<T, E>(
	func: impl AsyncFn() -> Result<T, E>,
	timeout: Duration,
	debounce: Duration,
) -> Result<T, E> {
	let start = CrossInstant::now();
	loop {
		match func().await {
			Ok(val) => return Ok(val),
			Err(err) => {
				if start.elapsed() > timeout {
					return Err(err);
				}
			}
		}
		crate::utils::sleep(debounce).await;
	}
}


/// Cross platform spawn_local function
#[cfg(target_arch = "wasm32")]
pub fn spawn_local<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut)
	// tokio::task::spawn_local(fut).await.expect("Task panicked")
}
/// Cross platform spawn_local function
#[cfg(all(not(target_arch="wasm32"),feature = "tokio"))]
pub fn spawn_local<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
where
	F: Future + 'static,
	F::Output: 'static,
{
	tokio::task::spawn_local(fut)
}
