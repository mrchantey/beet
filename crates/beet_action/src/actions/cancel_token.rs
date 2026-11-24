//! Whether anyone still awaits a call's output.
use alloc::sync::Arc;
use alloc::vec::Vec;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::task::Poll;
use core::task::Waker;

/// Resolves once a call is cancelled, ie the future awaiting its output was
/// dropped before the call resolved.
///
/// Carried by the call's [`OutHandler`](crate::prelude::OutHandler), so an
/// async action races its work against it and a cancelled call stops at its
/// next await. The token is read only: dropping the caller's future is the
/// one way to cancel. A call made with a handler of its own, ie
/// `commands.entity(entity).call(input, OutHandler::default())`, has no
/// future to drop and is [`never`](Self::never) cancelled.
#[derive(Debug, Default, Clone)]
pub struct CancelToken(Option<Arc<Mutex<CancelState>>>);

impl CancelToken {
	/// A token that is never cancelled.
	pub fn never() -> Self { Self(None) }

	/// Whether the call has been cancelled.
	pub fn is_cancelled(&self) -> bool {
		self.0
			.as_ref()
			.is_some_and(|state| state.lock().unwrap().cancelled)
	}

	/// Resolves once the call is cancelled, or never for
	/// [`never`](Self::never).
	pub fn cancelled(self) -> impl Future<Output = ()> {
		core::future::poll_fn(move |cx| {
			let Some(state) = &self.0 else {
				return Poll::Pending;
			};
			let mut state = state.lock().unwrap();
			if state.cancelled {
				return Poll::Ready(());
			}
			if !state.wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
				state.wakers.push(cx.waker().clone());
			}
			Poll::Pending
		})
	}
}

/// Held by the caller for as long as it awaits the output; dropping it
/// cancels every clone of its [`CancelToken`].
pub(crate) struct CancelGuard(Arc<Mutex<CancelState>>);

impl CancelGuard {
	/// A guard and the token its drop cancels.
	pub(crate) fn new() -> (Self, CancelToken) {
		let state = Arc::new(Mutex::new(CancelState::default()));
		(Self(state.clone()), CancelToken(Some(state)))
	}
}

impl Drop for CancelGuard {
	fn drop(&mut self) {
		let wakers = {
			let mut state = self.0.lock().unwrap();
			state.cancelled = true;
			core::mem::take(&mut state.wakers)
		};
		// woken after the lock is released, since a single threaded executor
		// may poll the woken task inline
		for waker in wakers {
			waker.wake();
		}
	}
}

#[derive(Debug, Default)]
struct CancelState {
	cancelled: bool,
	wakers: Vec<Waker>,
}
