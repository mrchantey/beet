use crate::prelude::*;
use core::future::Future;
#[cfg(feature = "std")]
use std::cell::Cell;
#[cfg(feature = "std")]
use std::pin::Pin;
#[cfg(feature = "std")]
use std::task;
#[cfg(feature = "std")]
use std::task::Poll;

#[cfg(feature = "std")]
thread_local! {
	/// Whether [`PanicContext::init`] has been called yet.
	/// Whether we are currently in a panic catch scope
	static IN_SCOPE: Cell<bool> = Cell::new(false);
	/// Captures the panic context
	static CONTEXT: Cell<Option<PanicContext>> = Cell::new(None);
	/// Whether the most recent panic hook invocation on this thread buffered
	/// the panic as escaped (fired outside any catch scope). Lets swallow
	/// sites like [`PanicContext::record_swallowed`] avoid double-recording.
	static ESCAPE_RECORDED: Cell<bool> = Cell::new(false);
}

#[cfg(feature = "std")]
static INITIALIZED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Panics that escaped every catch scope (or were swallowed by a task runner
/// before reaching one), stamped with the instant they fired. Attribution under
/// concurrent tests is genuinely ambiguous, so timeout reports include the
/// entries from their window unattributed rather than guessing an owner. The
/// buffer is never drained: concurrent suites each read the window they
/// observed via [`PanicContext::escaped_since`].
#[cfg(feature = "std")]
static ESCAPED: std::sync::Mutex<Vec<(Instant, String)>> =
	std::sync::Mutex::new(Vec::new());

/// Cross-platform method for capturing panic info, including in
/// non-unwind contexts like wasm. See [`Self::catch`]
#[cfg(feature = "std")]
pub(crate) struct PanicContext {
	/// The payload downcast to a string if possible
	payload: Option<String>,
	/// The file and linecol of the location if available
	location: Option<FileSpan>,
}

/// no_std (bare-metal) [`PanicContext`]: there is no unwinding under
/// `panic = abort`, so it cannot catch. [`Self::catch`] runs the test directly;
/// a failing assertion panics and the device panic handler logs it then
/// semihosting-exits (the abort-on-first-failure model).
#[cfg(not(feature = "std"))]
pub(crate) struct PanicContext;

#[cfg(not(feature = "std"))]
impl PanicContext {
	/// Runs `func`, mapping its result into a [`PanicResult`]. A panic is not
	/// caught here; it aborts via the panic handler.
	pub fn catch(func: impl FnOnce() -> Result<(), String>) -> PanicResult {
		match func() {
			Ok(()) => PanicResult::Ok,
			Err(err) => PanicResult::Err(err),
		}
	}

	/// Awaits `fut`, mapping its result into a [`PanicResult`]. As with
	/// [`Self::catch`], a panic aborts rather than being caught.
	pub fn catch_async<Fut>(fut: Fut) -> impl Future<Output = PanicResult>
	where
		Fut: Future<Output = Result<(), String>>,
	{
		async move {
			match fut.await {
				Ok(()) => PanicResult::Ok,
				Err(err) => PanicResult::Err(err),
			}
		}
	}

	/// No escape buffer under `panic = abort`: the first panic ends the run.
	pub fn escaped_since(_start: Instant) -> Vec<String> { Vec::new() }
}

#[cfg(feature = "std")]
impl PanicContext {
	/// Cross-platform method for capturing panic info, including in
	/// non-unwind contexts like wasm.
	///
	/// ## Note
	/// This method uses [`panic::set_hook`], calling the prev hook if
	/// a panic occurs outside of this scope. If another hook has overridden
	/// ours the report degrades to the unwind payload with no location.
	pub fn catch(func: impl FnOnce() -> Result<(), String>) -> PanicResult {
		match Self::catch_poll(|| Poll::Ready(func())) {
			Poll::Ready(result) => result,
			Poll::Pending => {
				unreachable!("catch should not return pending")
			}
		}
	}
	/// Cross-platform method for capturing panic info, including in
	/// non-unwind contexts like wasm.
	///
	/// ## Note
	/// This method uses [`panic::set_hook`], calling the prev hook if
	/// a panic occurs outside of this scope. If another hook has overridden
	/// ours the report degrades to the unwind payload with no location.
	pub fn catch_async<Fut>(fut: Fut) -> impl Future<Output = PanicResult>
	where
		Fut: Future<Output = Result<(), String>>,
	{
		PanicContextFuture::new(async move { fut.await })
	}

	/// Like [`Self::catch`] but supports [`Poll::Pending`] results
	fn catch_poll(
		func: impl FnOnce() -> Poll<Result<(), String>>,
	) -> Poll<PanicResult> {
		// 1. init scope
		if INITIALIZED.get().is_none() {
			Self::init();
		}
		// keep previous, incase nested context
		let prev_cx = CONTEXT.with(|cx| cx.take());
		let prev_scope = IN_SCOPE.with(|in_scope| in_scope.get());
		IN_SCOPE.with(|in_scope| in_scope.set(true));
		// 2. run function, normalizing the error to a fallback payload: the js
		// catch yields none, the native unwind carries the panic payload itself.
		// the wasm branch is interim until wasm can unwind; see the sunset note
		// on `js_runtime::catch_no_abort`
		#[cfg(target_arch = "wasm32")]
		let result = {
			let mut poll_result = None;
			let catch_result = js_runtime::catch_no_abort(|| {
				poll_result = Some(func());
				Ok(())
			});
			match catch_result {
				Ok(_) => Ok(poll_result.expect("func not called")),
				Err(()) => Err(None),
			}
		};
		#[cfg(not(target_arch = "wasm32"))]
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func))
			.map_err(|payload| display_ext::try_downcast_str(payload.as_ref()));

		// 3. map the result
		let result = match result {
			Ok(Poll::Ready(Ok(()))) => Poll::Ready(PanicResult::Ok),
			Ok(Poll::Ready(Err(err))) => Poll::Ready(PanicResult::Err(err)),
			Ok(Poll::Pending) => Poll::Pending,
			Err(fallback_payload) => {
				// prefer the hook-captured context for its location; a bypassed
				// hook (eg a dependency overrode it) degrades to the unwind
				// payload rather than panicking the runner into a second,
				// uncatchable panic that reports as the test's timeout
				let context =
					CONTEXT.with(|cx| cx.take()).unwrap_or(PanicContext {
						payload: fallback_payload,
						location: None,
					});
				Poll::Ready(PanicResult::Panic {
					payload: context.payload,
					location: context.location,
				})
			}
		};
		// 5. restore previous globals
		IN_SCOPE.with(|in_scope| in_scope.set(prev_scope));
		CONTEXT.with(|cx| cx.set(prev_cx));
		result
	}

	fn init() {
		INITIALIZED.get_or_init(|| true);
		let default_hook = std::panic::take_hook();

		std::panic::set_hook(Box::new(move |info| {
			let payload = display_ext::try_downcast_str(info.payload());
			let location = info.location().map(FileSpan::new_from_location);
			if IN_SCOPE.with(|in_scope| in_scope.get()) {
				// in a catch scope, capture context
				ESCAPE_RECORDED.with(|cell| cell.set(false));
				CONTEXT.with(|cx| {
					cx.set(Some(PanicContext { payload, location }));
				});
			} else {
				// not in a catch scope: report via the default hook and buffer
				// the escape so a timeout report can carry it
				default_hook(info);
				Self::buffer_escaped(payload, location);
				ESCAPE_RECORDED.with(|cell| cell.set(true));
			}
		}));
	}

	/// Formats and appends an escaped panic to the suite-wide buffer.
	///
	/// No-op until [`Self::init`] has installed the test hook, so a production
	/// app with panicking tasks never grows the buffer.
	fn buffer_escaped(payload: Option<String>, location: Option<FileSpan>) {
		if INITIALIZED.get().is_none() {
			return;
		}
		let payload =
			payload.unwrap_or_else(|| "opaque panic payload".to_string());
		let text = match location {
			Some(location) => format!("{payload} at {location}"),
			None => payload,
		};
		ESCAPED
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.push((Instant::now(), text));
	}

	/// Records a panic a task runner caught after it escaped every test catch
	/// scope (see `run_async_task_inner` / `tick_bridge_executor`).
	///
	/// Prefers the hook-captured context for its location, consuming it so the
	/// stale context cannot be misread later; skips entirely when the hook
	/// already buffered this panic as escaped. `fallback_payload` covers a
	/// bypassed hook (a dependency overrode it), where only the unwind payload
	/// survives.
	pub fn record_swallowed(fallback_payload: Option<String>) {
		if ESCAPE_RECORDED.with(|cell| cell.replace(false)) {
			return;
		}
		match CONTEXT.with(|cx| cx.take()) {
			Some(context) => {
				Self::buffer_escaped(context.payload, context.location)
			}
			None => Self::buffer_escaped(fallback_payload, None),
		}
	}

	/// Escaped panics recorded at or after `start`, ie a timeout report's
	/// window. Cloned rather than drained: attribution is ambiguous, so
	/// concurrent suites each report the window they observed.
	pub fn escaped_since(start: Instant) -> Vec<String> {
		ESCAPED
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.iter()
			.filter(|(at, _)| *at >= start)
			.map(|(_, text)| text.clone())
			.collect()
	}
}

/// Result of running code that may panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PanicResult {
	/// The operation completed successfully.
	Ok,
	/// The operation returned an error.
	Err(String),
	/// The operation panicked.
	Panic {
		/// The panic payload if it could be downcast to string.
		payload: Option<String>,
		/// The source location of the panic, if available.
		location: Option<FileSpan>,
	},
}
impl PanicResult {
	/// Returns `true` if the result is [`PanicResult::Ok`].
	pub fn is_ok(&self) -> bool { matches!(self, PanicResult::Ok) }
	/// Returns `true` if the result is [`PanicResult::Err`].
	pub fn is_err(&self) -> bool { matches!(self, PanicResult::Err(_)) }
	/// Returns `true` if the result is [`PanicResult::Panic`].
	pub fn is_panic(&self) -> bool { matches!(self, PanicResult::Panic { .. }) }
}

/// A future that wraps each poll in [`PanicContext::catch_poll`], to ensure
/// panics are properly handled in a cross-plaform way.
#[cfg(feature = "std")]
struct PanicContextFuture<F> {
	inner: F,
}

#[cfg(feature = "std")]
impl<F> PanicContextFuture<F> {
	pub fn new(inner: F) -> Self { Self { inner } }
}

#[cfg(feature = "std")]
impl<F: Future<Output = Result<(), String>>> Future for PanicContextFuture<F> {
	type Output = PanicResult;
	fn poll(
		self: Pin<&mut Self>,
		cx: &mut task::Context,
	) -> Poll<Self::Output> {
		// SAFETY: we never move out of the pinned field
		let inner = unsafe { self.map_unchecked_mut(|s| &mut s.inner) };

		PanicContext::catch_poll(|| inner.poll(cx))
	}
}

#[cfg(test)]
// wasm test runner uses PanicContext so cant test properly
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use crate::prelude::*;

	#[crate::test]
	fn works() {
		PanicContext::catch(|| Ok(())).xpect_eq(PanicResult::Ok);
		PanicContext::catch(|| Err("foobar".into()))
			.xpect_eq(PanicResult::Err("foobar".into()));
		PanicContext::catch(|| panic!("foobar")).xpect_eq(PanicResult::Panic {
			payload: Some("foobar".into()),
			location: Some(FileSpan::new_with_start(file!(), line!() - 2, 31)),
		});
	}

	/// REGRESSION: a bypassed hook (a dependency overrode it) must degrade to
	/// the unwind payload, not re-panic the runner into a timeout report.
	#[crate::test]
	fn survives_bypassed_hook() {
		// ensure our hook is installed before swapping it out
		PanicContext::catch(|| Ok(())).xpect_eq(PanicResult::Ok);
		let beet_hook = std::panic::take_hook();
		std::panic::set_hook(Box::new(|_| {}));
		let result = PanicContext::catch(|| panic!("foobar"));
		std::panic::set_hook(beet_hook);
		result.xpect_eq(PanicResult::Panic {
			payload: Some("foobar".into()),
			location: None,
		});
	}

	/// A panic swallowed by a nested catch (eg a task runner's `catch_unwind`)
	/// inside a catch scope must land in the escape buffer with its location,
	/// not vanish or attribute to the enclosing test.
	#[crate::test]
	fn buffers_swallowed_panics() {
		let start = Instant::now();
		let line = line!() + 2;
		PanicContext::catch(|| {
			std::panic::catch_unwind(|| panic!("swallowed pizza")).ok();
			PanicContext::record_swallowed(None);
			Ok(())
		})
		.xpect_eq(PanicResult::Ok);
		PanicContext::escaped_since(start)
			.iter()
			.any(|text| {
				text.contains("swallowed pizza")
					&& text.contains(&format!("{}:{line}", file!()))
			})
			.xpect_true();
	}

	/// A panic outside any catch scope (here a spawned thread) must be buffered
	/// by the hook itself, exactly once.
	#[crate::test]
	fn buffers_out_of_scope_panics() {
		// ensure the hook is installed
		PanicContext::catch(|| Ok(())).xpect_eq(PanicResult::Ok);
		let start = Instant::now();
		std::thread::spawn(|| panic!("thread pizza"))
			.join()
			.unwrap_err()
			.xmap(|payload| display_ext::try_downcast_str(payload.as_ref()))
			.xpect_eq(Some("thread pizza".to_string()));
		PanicContext::escaped_since(start)
			.iter()
			.filter(|text| text.contains("thread pizza"))
			.count()
			.xpect_eq(1);
	}

	#[crate::test]
	async fn works_async() {
		PanicContext::catch_async(async { Ok(()) })
			.await
			.xpect_eq(PanicResult::Ok);
		PanicContext::catch_async(async { Err("foobar".into()) })
			.await
			.xpect_eq(PanicResult::Err("foobar".into()));
		PanicContext::catch_async(async {
			async_ext::yield_now().await;
			async_ext::yield_now().await;
			async_ext::yield_now().await;
			panic!("foobar")
		})
		.await
		.xpect_eq(PanicResult::Panic {
			payload: Some("foobar".into()),
			location: Some(FileSpan::new_with_start(file!(), line!() - 5, 12)),
		});
	}
}
