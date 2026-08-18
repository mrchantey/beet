//! Async utilities and future helpers.
//!
//! Most helpers here are runtime drivers (`block_on`, `spawn`, `timeout`, the
//! shared `tokio` runtime) and are std-only. The no_std-capable pieces — the
//! boxed-future aliases and [`try_join_all`] — are ungated.

use crate::prelude::*;
use core::pin::Pin;
use core::task::Poll;

/// A future that never completes, parking the task forever.
///
/// Used by a long-running action that hands the process lifetime to a spawned
/// server: it triggers the start, then yields here so the action future never
/// resolves and the server decides when to exit. no_std.
pub async fn yield_forever<T>() -> T { core::future::pending().await }

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

/// Polls a collection of futures concurrently, resolving to their outputs in
/// iteration order once all complete.
///
/// A no_std drop-in for `futures::future::join_all`, backed only by
/// `alloc` + `core`.
pub async fn join_all<Fut, T>(
	futures: impl IntoIterator<Item = Fut>,
) -> Vec<T>
where
	Fut: Future<Output = T>,
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
				Poll::Ready(value) => {
					results[idx] = Some(value);
					*slot = None;
				}
				Poll::Pending => all_done = false,
			}
		}
		if all_done {
			Poll::Ready(
				results.iter_mut().map(|r| r.take().unwrap()).collect(),
			)
		} else {
			Poll::Pending
		}
	})
	.await
}

/// Like [`join_all`], but with at most `limit` futures in flight at once.
///
/// A future does no work until it is first polled, so this admits a new future
/// only as an earlier one completes. Prefer it for any fan-out over a remote
/// store: an unbounded [`join_all`] over thousands of keys opens thousands of
/// concurrent connections at once, and every one of them times out.
///
/// A `limit` of `0` is treated as `1`. no_std.
pub async fn join_all_bounded<Fut, T>(
	limit: usize,
	futures: impl IntoIterator<Item = Fut>,
) -> Vec<T>
where
	Fut: Future<Output = T>,
{
	let limit = limit.max(1);
	let mut futures: Vec<Option<Pin<Box<Fut>>>> =
		futures.into_iter().map(|fut| Some(Box::pin(fut))).collect();
	let mut results: Vec<Option<T>> = core::iter::repeat_with(|| None)
		.take(futures.len())
		.collect();
	// exclusive high-water mark of futures admitted, and how many of those are
	// still pending
	let mut admitted = 0;
	let mut in_flight = 0;

	core::future::poll_fn(move |cx| {
		loop {
			// fill the free slots from the tail of the queue
			while in_flight < limit && admitted < futures.len() {
				admitted += 1;
				in_flight += 1;
			}
			let mut completed = false;
			for idx in 0..admitted {
				let Some(fut) = futures[idx].as_mut() else {
					continue;
				};
				if let Poll::Ready(value) = fut.as_mut().poll(cx) {
					results[idx] = Some(value);
					futures[idx] = None;
					in_flight -= 1;
					completed = true;
				}
			}
			if in_flight == 0 && admitted == futures.len() {
				return Poll::Ready(
					results.iter_mut().map(|slot| slot.take().unwrap()).collect(),
				);
			}
			// a completion freed a slot, so loop round to admit its replacement
			if !completed {
				return Poll::Pending;
			}
		}
	})
	.await
}

/// Like [`try_join_all`], but with at most `limit` futures in flight at once.
///
/// Unlike [`try_join_all`] this does NOT short-circuit: every admitted future
/// runs to completion and the first [`Err`] in iteration order is returned. no_std.
pub async fn try_join_all_bounded<Fut, T, E>(
	limit: usize,
	futures: impl IntoIterator<Item = Fut>,
) -> Result<Vec<T>, E>
where
	Fut: Future<Output = Result<T, E>>,
{
	join_all_bounded(limit, futures).await.into_iter().collect()
}

/// A 'static + Send, making it suitable for spawning on async runtimes
pub type SendBoxedFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;
/// A 'static + Send + Sync boxed [`Future`], required where the future is held
/// across an await inside a `Send + Sync` value (eg a body stream awaiting
/// [`time_ext::sleep`](crate::prelude::time_ext::sleep)).
pub type SendSyncBoxedFuture<T> =
	Pin<Box<dyn 'static + Send + Sync + Future<Output = T>>>;
/// A 'static + Send, making it suitable for spawning on async runtimes
pub type LifetimeSendBoxedFuture<'a, T> =
	Pin<Box<dyn 'a + Send + Future<Output = T>>>;
/// A boxed [`Future`] that is never `Send`, for hooks always polled on the
/// thread they were created on (eg the thread-local server/runtime layer).
pub type LocalBoxedFuture<'a, T> = Pin<Box<dyn 'a + Future<Output = T>>>;

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
		// generous timeout vs tiny work: stays green even when the wasm event loop
		// is starved under heavy parallel test load.
		async_ext::timeout(Duration::from_secs(5), async {
			time_ext::sleep(Duration::from_millis(10)).await;
			42
		})
		.await
		.unwrap()
		.xpect_eq(42);
	}

	/// The bounded join runs every future and keeps iteration order, while never
	/// exceeding `limit` in flight. Regression guard: an unbounded fan-out over a
	/// remote store starts thousands of requests at once, and every one past the
	/// connection pool fails its connect timeout.
	#[crate::test]
	async fn join_all_bounded_caps_in_flight() {
		use std::sync::Arc;
		use std::sync::atomic::AtomicUsize;
		use std::sync::atomic::Ordering;

		let in_flight = Arc::new(AtomicUsize::new(0));
		let peak = Arc::new(AtomicUsize::new(0));
		let total = 20;
		let futures = (0..total).map(|idx| {
			let in_flight = in_flight.clone();
			let peak = peak.clone();
			async move {
				let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
				peak.fetch_max(now, Ordering::SeqCst);
				// yield long enough that an unbounded join would overlap all of them
				for _ in 0..4 {
					async_ext::yield_now().await;
				}
				in_flight.fetch_sub(1, Ordering::SeqCst);
				idx
			}
		});
		async_ext::join_all_bounded(4, futures)
			.await
			.xpect_eq((0..total).collect::<Vec<_>>());
		peak.load(Ordering::SeqCst).xpect_eq(4);
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
