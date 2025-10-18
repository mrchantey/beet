pub use futures::future::try_join_all;
use futures_lite::future::YieldNow;
use std::pin::Pin;
use std::time::Duration;

use crate::prelude::*;


pub fn block_on<F: Future>(fut: F) -> F::Output {
	futures::executor::block_on(fut)
}
pub fn yield_now() -> YieldNow { futures_lite::future::yield_now() }

/// A 'static + Send, making it suitable for use-cases like tokio::spawn
pub type SendBoxedFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;
/// A 'static + Send, making it suitable for use-cases like tokio::spawn
pub type LifetimeSendBoxedFuture<'a, T> =
	Pin<Box<dyn 'a + Send + Future<Output = T>>>;

/// A BoxedFuture which is `Send` on non-wasm32 targets with multi_threaded enabled
#[cfg(target_arch = "wasm32")]
pub type MaybeSendBoxedFuture<'a, T> = Pin<Box<dyn 'a + Future<Output = T>>>;
/// A BoxedFuture which is `Send` on non-wasm32 targets with multi_threaded enabled
#[cfg(not(target_arch = "wasm32"))]
pub type MaybeSendBoxedFuture<'a, T> =
	Pin<Box<dyn 'a + Send + Future<Output = T>>>;

/// Cross platform spawn_local function
#[cfg(target_arch = "wasm32")]
pub fn spawn_local<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut)
}
/// Cross platform spawn_local function
// TODO deprecate for async-executor or bevy tasks?
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
pub fn spawn_local<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
where
	F: Future + 'static,
	F::Output: 'static,
{
	tokio::task::spawn_local(fut)
}


#[cfg(all(not(target_arch = "wasm32"), not(feature = "tokio")))]
pub fn spawn_local<F>(_: F)
where
	F: Future<Output = ()> + 'static,
{
	unimplemented!("please enable tokio feature")
}
/// Cross platform spawn_local function
#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut)
}
/// Cross platform spawn function
// TODO deprecate for async-executor or bevy tasks?
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
pub fn spawn<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
where
	F: Future + 'static + Send,
	F::Output: 'static + Send,
{
	tokio::task::spawn(fut)
}


#[cfg(all(not(target_arch = "wasm32"), not(feature = "tokio")))]
pub fn spawn<F>(_: F)
where
	F: Future<Output = ()> + 'static,
{
	unimplemented!("please enable tokio feature")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "operation timed out")
	}
}

impl std::error::Error for TimeoutError {}

/// Await a future with a timeout
pub async fn timeout<F: Future>(
	duration: Duration,
	fut: F,
) -> Result<F::Output, TimeoutError> {
	use futures_lite::future::race;

	race(
		async move {
			time_ext::sleep(duration).await;
			Err(TimeoutError)
		},
		async move { Ok(fut.await) },
	)
	.await
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn timeout_completes_before_timeout() {
		async_ext::timeout(Duration::from_millis(100), async {
			time_ext::sleep(Duration::from_millis(10)).await;
			42
		})
		.await
		.unwrap()
		.xpect_eq(42);
	}

	#[sweet::test]
	async fn timeout_exceeds_timeout() {
		async_ext::timeout(Duration::from_millis(10), async {
			time_ext::sleep(Duration::from_millis(100)).await;
			42
		})
		.await
		.unwrap_err()
		.xpect_eq(async_ext::TimeoutError);
	}
}
