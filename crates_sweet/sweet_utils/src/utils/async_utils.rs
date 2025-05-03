use std::time::Duration;

pub struct AsyncUtils;

impl AsyncUtils {
	/// Retries a function until it returns Ok or the timeout is reached.
	pub async fn retry_async<T, E>(
		func: impl AsyncFn() -> Result<T, E>,
		timeout: Duration,
		debounce: Duration,
	) -> Result<T, E> {
		let start = std::time::Instant::now();
		loop {
			match func().await {
				Ok(val) => return Ok(val),
				Err(err) => {
					if start.elapsed() > timeout {
						return Err(err);
					}
				}
			}
			tokio::time::sleep(debounce).await;
		}
	}
}
