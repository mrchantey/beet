//! Async utilities and future helpers.

use bevy::tasks::IoTaskPool;
use bevy::tasks::Task;
pub use futures::future::try_join_all;
use futures_lite::future::YieldNow;
use std::pin::Pin;
use std::time::Duration;


use crate::prelude::*;

/// Blocks the current thread on a future until it completes.
pub fn block_on<F: Future>(fut: F) -> F::Output {
	futures::executor::block_on(fut)
}

/// Blocks the current thread on a future, running it on a [`LocalExecutor`].
///
/// This is the underlying driver for [`#[beet::main]`](beet_core_macros::beet_main).
///
/// [`LocalExecutor`]: async_executor::LocalExecutor
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub fn block_on_local_executor<F: Future>(fut: F) -> F::Output {
	let ex = async_executor::LocalExecutor::new();
	futures_lite::future::block_on(ex.run(fut))
}
/// Yields execution back to the async runtime.
pub fn yield_now() -> YieldNow { futures_lite::future::yield_now() }

/// A 'static + Send, making it suitable for spawning on async runtimes
pub type SendBoxedFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;
/// A 'static + Send, making it suitable for spawning on async runtimes
pub type LifetimeSendBoxedFuture<'a, T> =
	Pin<Box<dyn 'a + Send + Future<Output = T>>>;

/// A BoxedFuture which is `Send` on non-wasm32 targets with bevy_multithreaded enabled
#[cfg(target_arch = "wasm32")]
pub type MaybeSendBoxedFuture<'a, T> = Pin<Box<dyn 'a + Future<Output = T>>>;
/// A BoxedFuture which is `Send` on non-wasm32 targets with bevy_multithreaded enabled
#[cfg(not(target_arch = "wasm32"))]
pub type MaybeSendBoxedFuture<'a, T> =
	Pin<Box<dyn 'a + Send + Future<Output = T>>>;


/// Cross platform spawn_local function
pub fn spawn_local<F>(fut: F) -> Task<F::Output>
where
	F: Future + 'static,
	F::Output: 'static + MaybeSend + MaybeSync,
{
	IoTaskPool::get().spawn_local(fut)
}

/// Cross platform spawn function
pub fn spawn<F>(fut: F) -> Task<F::Output>
where
	F: Future + 'static + MaybeSend + MaybeSync,
	F::Output: 'static + MaybeSend + MaybeSync,
{
	IoTaskPool::get().spawn(fut)
}

/// Error returned when an async operation times out.
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

/// Shared multi-threaded tokio runtime, lazily initialized.
///
/// Several dependencies like `reqwest` and AWS SDKs require a tokio
/// runtime. This provides a single cached runtime so we can bridge
/// their futures into beet's async-executor based runtime.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub fn tokio() -> &'static tokio::runtime::Runtime {
	static TOKIO: std::sync::LazyLock<tokio::runtime::Runtime> =
		std::sync::LazyLock::new(|| {
			tokio::runtime::Builder::new_multi_thread()
				.enable_all()
				.build()
				.expect("failed to build tokio runtime")
		});
	&TOKIO
}

/// Spawn a future on the shared tokio runtime and await its completion.
///
/// Use this to bridge tokio-dependent code (reqwest, AWS SDK, etc.)
/// into non-tokio async contexts.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub async fn on_tokio<F, T>(future: F) -> Result<T, BevyError>
where
	F: 'static + Send + Future<Output = Result<T, BevyError>>,
	T: 'static + Send,
{
	tokio()
		.spawn(future)
		.await
		.map_err(|err| bevyhow!("tokio task panicked: {err}"))?
}

/// Convenience wrapper that pins an [`on_tokio`] future into a [`SendBoxedFuture`].
///
/// Replaces the common pattern `Box::pin(async_ext::on_tokio(async move { ... }))`.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub fn pin_tokio<F, T>(future: F) -> SendBoxedFuture<Result<T, BevyError>>
where
	F: 'static + Send + Future<Output = Result<T, BevyError>>,
	T: 'static + Send,
{
	Box::pin(on_tokio(future))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn timeout_completes_before_timeout() {
		async_ext::timeout(Duration::from_millis(500), async {
			time_ext::sleep(Duration::from_millis(10)).await;
			42
		})
		.await
		.unwrap()
		.xpect_eq(42);
	}

	#[crate::test]
	async fn timeout_exceeds_timeout() {
		async_ext::timeout(Duration::from_millis(10), async {
			time_ext::sleep(Duration::from_millis(1000)).await;
			42
		})
		.await
		.unwrap_err()
		.xpect_eq(async_ext::TimeoutError);
	}
}
