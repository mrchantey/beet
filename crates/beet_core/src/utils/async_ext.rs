//! Async utilities and future helpers.
//!
//! Most helpers here are runtime drivers (`block_on`, `spawn`, `timeout`, the
//! shared `tokio` runtime) and are std-only. The no_std-capable pieces — the
//! boxed-future aliases and [`try_join_all`] — are ungated.

use crate::prelude::*;
use core::pin::Pin;
use core::task::Poll;

/// Polls a collection of fallible futures concurrently, resolving to their
/// outputs in iteration order once all succeed, or short-circuiting on the
/// first [`Err`].
///
/// A no_std drop-in for `futures::future::try_join_all`, backed only by
/// `alloc` + `core`.
pub async fn try_join_all<Fut, T, E>(
	futures: impl IntoIterator<Item = Fut>,
) -> Result<Vec<T>, E>
where
	Fut: Future<Output = Result<T, E>>,
{
	let mut futures: Vec<Option<Pin<Box<Fut>>>> =
		futures.into_iter().map(|fut| Some(Box::pin(fut))).collect();
	let mut results: Vec<Option<T>> = core::iter::repeat_with(|| None)
		.take(futures.len())
		.collect();

	core::future::poll_fn(move |cx| {
		let mut all_done = true;
		for (idx, slot) in futures.iter_mut().enumerate() {
			let Some(fut) = slot else { continue };
			match fut.as_mut().poll(cx) {
				Poll::Ready(Ok(value)) => {
					results[idx] = Some(value);
					*slot = None;
				}
				Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
				Poll::Pending => all_done = false,
			}
		}
		if all_done {
			Poll::Ready(Ok(results
				.iter_mut()
				.map(|r| r.take().unwrap())
				.collect()))
		} else {
			Poll::Pending
		}
	})
	.await
}

/// A 'static + Send, making it suitable for spawning on async runtimes
pub type SendBoxedFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;
/// A 'static + Send, making it suitable for spawning on async runtimes
pub type LifetimeSendBoxedFuture<'a, T> =
	Pin<Box<dyn 'a + Send + Future<Output = T>>>;

cfg_if! {
	// `Send` only in multi-threaded native builds, matching [`MaybeSend`].
	if #[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))] {
		/// A boxed [`Future`], `Send` only in multi-threaded native builds (matching [`MaybeSend`]).
		pub type MaybeSendBoxedFuture<'a, T> =
			Pin<Box<dyn 'a + Send + Future<Output = T>>>;
	} else {
		/// A boxed [`Future`], `Send` only in multi-threaded native builds (matching [`MaybeSend`]).
		pub type MaybeSendBoxedFuture<'a, T> = Pin<Box<dyn 'a + Future<Output = T>>>;
	}
}

/// Yields execution back to the async runtime.
#[cfg(feature = "std")]
pub fn yield_now() -> futures_lite::future::YieldNow {
	futures_lite::future::yield_now()
}

/// Blocks the current thread on a future until it completes.
///
/// std-only and infallible: it owns a real executor, so it drives a future that
/// pends to completion. no_std has no executor; reach for [`try_block_on`] there.
#[cfg(feature = "std")]
pub fn block_on<F: Future>(fut: F) -> F::Output {
	futures::executor::block_on(fut)
}

/// Drives an *immediately-ready* future to completion by polling once with a
/// no-op waker, returning [`Err`] if it pends.
///
/// The no_std-capable counterpart to [`block_on`]: with no executor it cannot
/// park a pending future, so a future that pends is an error rather than a hang.
/// Suits a future that is `async` only for a seam yet resolves in one poll (eg
/// in-memory schema validation, whose async is reserved for the remote-fetch
/// path).
pub fn try_block_on<F: Future>(fut: F) -> Result<F::Output> {
	use core::task::Context;
	use core::task::Poll;
	let waker = core::task::Waker::noop();
	let mut cx = Context::from_waker(waker);
	let mut fut = fut;
	// SAFETY: `fut` is owned and never moved after pinning.
	let mut fut = unsafe { core::pin::Pin::new_unchecked(&mut fut) };
	match fut.as_mut().poll(&mut cx) {
		Poll::Ready(output) => Ok(output),
		Poll::Pending => {
			bevybail!("try_block_on requires an immediately-ready future")
		}
	}
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

/// Cross platform spawn_local function
#[cfg(feature = "std")]
pub fn spawn_local<F>(fut: F) -> bevy::tasks::Task<F::Output>
where
	F: Future + 'static,
	F::Output: 'static + MaybeSend + MaybeSync,
{
	bevy::tasks::IoTaskPool::get().spawn_local(fut)
}

/// Cross platform spawn function
#[cfg(feature = "std")]
pub fn spawn<F>(fut: F) -> bevy::tasks::Task<F::Output>
where
	F: Future + 'static + MaybeSend + MaybeSync,
	F::Output: 'static + MaybeSend + MaybeSync,
{
	cfg_if! {
		// `IoTaskPool::spawn` requires `Send` whenever bevy's `multi_threaded`
		// feature is active; only here is the future guaranteed `Send` (matching
		// [`MaybeSend`]). Otherwise spawn locally, which never requires it.
		if #[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))] {
			bevy::tasks::IoTaskPool::get().spawn(fut)
		} else {
			bevy::tasks::IoTaskPool::get().spawn_local(fut)
		}
	}
}

/// Error returned when an async operation times out.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

#[cfg(feature = "std")]
impl core::fmt::Display for TimeoutError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "operation timed out")
	}
}

#[cfg(feature = "std")]
impl core::error::Error for TimeoutError {}

/// Await a future with a timeout
#[cfg(feature = "std")]
pub async fn timeout<F: Future>(
	duration: std::time::Duration,
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
