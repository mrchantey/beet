use crate::prelude::*;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task;
use std::task::Poll;

thread_local! {
	/// Whether [`PanicContext::init`] has been called yet.
	/// Whether we are currently in a panic catch scope
	static IN_SCOPE: Cell<bool> = Cell::new(false);
	/// Captures the panic context
	static CONTEXT: Cell<Option<PanicContext>> = Cell::new(None);
}

static INITIALIZED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Cross-platform method for capturing panic info, including in
/// non-unwind contexts like wasm. See [`Self::catch`]
pub struct PanicContext {
	/// The payload downcast to a string if possible
	payload: Option<String>,
	/// The file and linecol of the location if available
	location: Option<FileSpan>,
}

impl PanicContext {
	/// Cross-platform method for capturing panic info, including in
	/// non-unwind contexts like wasm.
	///
	/// ## Note
	/// This method uses [`panic::set_hook`], calling the prev hook if
	/// a panic occurs outside of this scope. Overriding set_hook will break
	/// this method.
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
	/// a panic occurs outside of this scope. Overriding set_hook will break
	/// this method.
	pub fn catch_async<Fut>(
		func: impl FnOnce() -> Fut,
	) -> impl Future<Output = PanicResult>
	where
		Fut: Future<Output = Result<(), String>>,
	{
		PanicContextFuture::new(async move { func().await })
	}

	/// Like [`Self::catch`] but supports [`Poll::Pending`] results
	fn catch_poll(
		func: impl FnOnce() -> Poll<Result<(), String>>,
	) -> Poll<PanicResult> {
		// 1. init scope
		if INITIALIZED.get().is_none() {
			Self::init();
		}
		CONTEXT.with(|cx| {
			cx.set(None);
		});
		IN_SCOPE.with(|in_scope| in_scope.set(true));
		// 2. run function
		#[cfg(target_arch = "wasm32")]
		let result = {
			let mut poll_result = None;
			let catch_result = js_runtime::catch_no_abort(|| {
				poll_result = Some(func());
				Ok(())
			});
			match catch_result {
				Ok(_) => Ok(poll_result.expect("func not called")),
				Err(()) => Err(()),
			}
		};
		#[cfg(not(target_arch = "wasm32"))]
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func));
		// 3. reset scope
		IN_SCOPE.with(|in_scope| in_scope.set(false));

		// 4. map and return the result
		match result {
			Ok(Poll::Ready(Ok(()))) => Poll::Ready(PanicResult::Ok),
			Ok(Poll::Ready(Err(err))) => Poll::Ready(PanicResult::Err(err)),
			Ok(Poll::Pending) => Poll::Pending,
			Err(_) => {
				crate::breakpoint!();
				// panicked
				let context = CONTEXT.with(|cx| {
					cx.take().expect(
						"panic without context, has the panic hook been overridden?",
					)
				});
				crate::breakpoint!();
				Poll::Ready(PanicResult::Panic {
					payload: context.payload,
					location: context.location,
				})
			}
		}
	}

	fn init() {
		INITIALIZED.get_or_init(|| true);
		let default_hook = std::panic::take_hook();

		std::panic::set_hook(Box::new(move |info| {
			if !IN_SCOPE.with(|in_scope| in_scope.get()) {
				// not in a catch scope, use default hook
				default_hook(info);
				return;
			}
			CONTEXT.with(|cx| {
				let payload = downcast_str(info.payload());
				let location = info.location().map(FileSpan::new_from_location);
				cx.set(Some(PanicContext { payload, location }));
			});
		}));
	}
}
/// Attempt to downcast a panic payload into a string
fn downcast_str(payload: &dyn std::any::Any) -> Option<String> {
	if let Some(str) = payload.downcast_ref::<&str>() {
		Some(str.to_string())
	} else if let Some(str) = payload.downcast_ref::<String>() {
		Some(str.clone())
	} else {
		None
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanicResult {
	Ok,
	Err(String),
	Panic {
		payload: Option<String>,
		location: Option<FileSpan>,
	},
}


/// A future that wraps each poll in [`PanicContext::catch_poll`], to ensure
/// panics are properly handled in a cross-plaform way.
struct PanicContextFuture<F> {
	inner: F,
}

impl<F> PanicContextFuture<F> {
	pub fn new(inner: F) -> Self { Self { inner } }
}

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
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		PanicContext::catch(|| Ok(())).xpect_eq(PanicResult::Ok);
		// PanicContext::catch(|| Err("foobar".into()))
		// 	.xpect_eq(PanicResult::Err("foobar".into()));
		// PanicContext::catch(|| panic!("foobar")).xpect_eq(PanicResult::Panic {
		// 	payload: Some("foobar".into()),
		// 	location: Some(FileSpan::new_with_start(file!(), line!() - 2, 32)),
		// });
	}
	// #[sweet::test]
	// async fn works_async() {
	// 	PanicContext::catch_async(async || Ok(()))
	// 		.await
	// 		.xpect_eq(PanicResult::Ok);
	// 	PanicContext::catch_async(async || Err("foobar".into()))
	// 		.await
	// 		.xpect_eq(PanicResult::Err("foobar".into()));
	// 	PanicContext::catch_async(async || {
	// 		async_ext::yield_now().await;
	// 		async_ext::yield_now().await;
	// 		async_ext::yield_now().await;
	// 		panic!("foobar")
	// 	})
	// 	.await
	// 	.xpect_eq(PanicResult::Panic {
	// 		payload: Some("foobar".into()),
	// 		location: Some(FileSpan::new_with_start(file!(), line!() - 5, 13)),
	// 	});
	// }
}
