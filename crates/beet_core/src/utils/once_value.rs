//! A one-shot value channel, usable on `no_std`.
//!
//! A single value is published once via [`OnceValue::signal`] and the awaiting
//! task ([`OnceValueRx::wait`]) is woken. Every primitive is no_std-capable
//! through [`bevy::platform::sync`] + `core::*`, so this is the channel of
//! choice for async result hand-offs that must run on bare-metal targets.

use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::task::Poll;
use core::task::Waker;

/// The sending half of a [`OnceValue::oneshot`] channel.
pub struct OnceValue<T>(Arc<Mutex<OnceState<T>>>);
/// The receiving half of a [`OnceValue::oneshot`] channel.
pub struct OnceValueRx<T>(Arc<Mutex<OnceState<T>>>);

/// The value and the waker share one lock, so a [`OnceValue::signal`] racing a
/// poll on another thread either lands before the poll reads the value or
/// after it stores its waker, never between the two.
struct OnceState<T> {
	value: Option<T>,
	waker: Option<Waker>,
}

impl<T> OnceValue<T> {
	/// Creates a one-shot value channel, returning the [`OnceValue`] sender and
	/// [`OnceValueRx`] receiver halves.
	pub fn oneshot() -> (OnceValue<T>, OnceValueRx<T>) {
		let state = Arc::new(Mutex::new(OnceState {
			value: None,
			waker: None,
		}));
		(OnceValue(state.clone()), OnceValueRx(state))
	}

	/// Publishes the value and wakes the awaiting task.
	pub fn signal(self, value: T) {
		let waker = {
			let mut state = self.0.lock().unwrap();
			state.value = Some(value);
			state.waker.take()
		};
		// wake after releasing the lock: a single-threaded executor (wasm) polls
		// the woken task inline, and that poll re-enters `wait`, which takes it.
		if let Some(waker) = waker {
			waker.wake();
		}
	}
}

impl<T> OnceValueRx<T> {
	/// Resolves once [`OnceValue::signal`] has published a value.
	pub fn wait(self) -> impl Future<Output = T> {
		core::future::poll_fn(move |cx| {
			let mut state = self.0.lock().unwrap();
			match state.value.take() {
				Some(value) => Poll::Ready(value),
				None => {
					state.waker = Some(cx.waker().clone());
					Poll::Pending
				}
			}
		})
	}
}

#[cfg(all(test, feature = "std", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use core::sync::atomic::AtomicBool;
	use core::sync::atomic::Ordering;
	use core::task::Context;
	use core::task::Waker;
	use std::sync::Arc;
	use std::sync::Barrier;
	use std::task::Wake;

	/// Records whether it was woken.
	struct FlagWaker(AtomicBool);

	impl Wake for FlagWaker {
		fn wake(self: Arc<Self>) { self.0.store(true, Ordering::SeqCst); }
	}

	/// A signal racing the receiver's poll from another thread must either be
	/// seen by that poll or wake the waker it stored, never neither.
	#[crate::test]
	fn cross_thread_signal_never_lost() {
		for _ in 0..20_000 {
			let (send, recv) = OnceValue::oneshot();
			let barrier = Arc::new(Barrier::new(2));
			let signaler = {
				let barrier = barrier.clone();
				std::thread::spawn(move || {
					barrier.wait();
					send.signal(());
				})
			};
			let flag = Arc::new(FlagWaker(AtomicBool::new(false)));
			let waker = Waker::from(flag.clone());
			let mut fut = core::pin::pin!(recv.wait());
			barrier.wait();
			let ready = fut
				.as_mut()
				.poll(&mut Context::from_waker(&waker))
				.is_ready();
			signaler.join().unwrap();
			(ready || flag.0.load(Ordering::SeqCst)).xpect_true();
		}
	}
}
