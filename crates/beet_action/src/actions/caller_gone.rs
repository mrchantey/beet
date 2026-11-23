//! Whether anyone still awaits a call's output.
use alloc::sync::Arc;
use alloc::vec::Vec;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::task::Poll;
use core::task::Waker;

/// Resolves once the caller awaiting a call's output is gone, ie its future
/// was dropped before the call resolved.
///
/// Carried by the call's [`OutHandler`](crate::prelude::OutHandler), so an
/// async action races its work against it and a dropped call stops at its
/// next await. A call made with a handler of its own, ie
/// `commands.entity(entity).call(input, OutHandler::default())`, has nobody
/// to leave and is [`never`](Self::never) gone.
#[derive(Debug, Default, Clone)]
pub struct CallerGone(Option<Arc<Mutex<GoneState>>>);

impl CallerGone {
	/// A caller that never leaves.
	pub fn never() -> Self { Self(None) }

	/// Whether the caller has gone.
	pub fn is_gone(&self) -> bool {
		self.0
			.as_ref()
			.is_some_and(|state| state.lock().unwrap().gone)
	}

	/// Resolves once the caller has gone, or never for [`never`](Self::never).
	pub fn wait(self) -> impl Future<Output = ()> {
		core::future::poll_fn(move |cx| {
			let Some(state) = &self.0 else {
				return Poll::Pending;
			};
			let mut state = state.lock().unwrap();
			if state.gone {
				return Poll::Ready(());
			}
			if !state.wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
				state.wakers.push(cx.waker().clone());
			}
			Poll::Pending
		})
	}
}

/// Held by the caller for as long as it awaits the output; dropping it makes
/// every clone of its [`CallerGone`] resolve.
pub(crate) struct CallerGuard(Arc<Mutex<GoneState>>);

impl CallerGuard {
	/// A guard and the signal its drop resolves.
	pub(crate) fn new() -> (Self, CallerGone) {
		let state = Arc::new(Mutex::new(GoneState::default()));
		(Self(state.clone()), CallerGone(Some(state)))
	}
}

impl Drop for CallerGuard {
	fn drop(&mut self) {
		let wakers = {
			let mut state = self.0.lock().unwrap();
			state.gone = true;
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
struct GoneState {
	gone: bool,
	wakers: Vec<Waker>,
}
