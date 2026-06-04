//! A one-shot value channel, usable on `no_std`.
//!
//! A single value is published once via [`OnceValue::signal`] and the awaiting
//! task ([`OnceValueRx::wait`]) is woken. Every primitive is no_std-capable
//! through [`bevy::platform::sync`] + `core::*`, so this is the channel of
//! choice for async result hand-offs that must run on bare-metal targets.

use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use core::task::Poll;
use core::task::Waker;

/// Creates a one-shot value channel, returning the [`OnceValue`] sender and
/// [`OnceValueRx`] receiver halves.
pub fn oneshot<T>() -> (OnceValue<T>, OnceValueRx<T>) {
	let inner = Arc::new(OnceValueInner {
		value: Mutex::new(None),
		waker: Mutex::new(None),
		set: AtomicBool::new(false),
	});
	(OnceValue(inner.clone()), OnceValueRx(inner))
}

/// The sending half of a [`oneshot`] channel.
pub struct OnceValue<T>(Arc<OnceValueInner<T>>);
/// The receiving half of a [`oneshot`] channel.
pub struct OnceValueRx<T>(Arc<OnceValueInner<T>>);

struct OnceValueInner<T> {
	value: Mutex<Option<T>>,
	waker: Mutex<Option<Waker>>,
	set: AtomicBool,
}

impl<T> OnceValue<T> {
	/// Publishes the value and wakes the awaiting task.
	pub fn signal(self, value: T) {
		*self.0.value.lock().unwrap() = Some(value);
		self.0.set.store(true, Ordering::SeqCst);
		if let Some(waker) = self.0.waker.lock().unwrap().take() {
			waker.wake();
		}
	}
}

impl<T> OnceValueRx<T> {
	/// Resolves once [`OnceValue::signal`] has published a value.
	pub fn wait(self) -> impl Future<Output = T> {
		core::future::poll_fn(move |cx| {
			if self.0.set.load(Ordering::SeqCst) {
				Poll::Ready(self.0.value.lock().unwrap().take().unwrap())
			} else {
				*self.0.waker.lock().unwrap() = Some(cx.waker().clone());
				Poll::Pending
			}
		})
	}
}
