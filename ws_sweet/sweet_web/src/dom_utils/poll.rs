use crate::*;
use anyhow::Result;
use std::time::Duration;

pub async fn poll_ok<T>(f: impl FnMut() -> Result<T>) -> Result<T> {
	poll_ok_with_timeout(f, Duration::from_secs(2)).await
}

pub async fn poll_ok_with_timeout<T>(
	mut f: impl FnMut() -> Result<T>,
	timeout: Duration,
) -> Result<T> {
	let start = performance_now();
	loop {
		match f() {
			Ok(val) => return Ok(val),
			Err(err) => {
				if performance_now() - start > timeout.as_millis() as f64 {
					return Err(err);
				}
				wait_for_millis(10).await;
			}
		}
	}
}
