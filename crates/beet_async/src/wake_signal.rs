// The condvar handshake requires real OS threads. On `no_std` and on wasm
// (where `std::sync::Condvar::wait` panics — there are no threads to notify it)
// the handshake is a no-op: those targets drive futures synchronously via
// `spawn_local` + local task-pool ticking before the waiter is reached.
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use bevy::platform::sync::Arc;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use bevy::platform::sync::Mutex;

/// [`WakeSignaler`] is a custom signaling primitive used in order to fulfill our specific requirements for
/// our async bridge. We need to wait at the sync point, after waking all the futures and only when
/// all the futures have had a chance to run we stop waiting.
/// We need this signaling to occur also if the future is dropped, or if the future panics
/// so we implement the signaling *on* the Drop implementation.
/// This also makes replacing the wake signal automatically drop and signal the previous one.
pub(crate) struct WakeSignaler(
	#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
	Arc<(Mutex<bool>, std::sync::Condvar)>,
	#[cfg(not(all(feature = "std", not(target_arch = "wasm32"))))] (),
);
/// Counterpart to the [`WakeSignaler`], the [`WakeWaiter`] waits for the [`WakeSignaler`] to drop and notify.
pub(crate) struct WakeWaiter(
	#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
	Arc<(Mutex<bool>, std::sync::Condvar)>,
	#[cfg(not(all(feature = "std", not(target_arch = "wasm32"))))] (),
);

#[inline]
pub(crate) fn pair() -> (WakeSignaler, WakeWaiter) {
	#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
	let inner = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
	#[cfg(not(all(feature = "std", not(target_arch = "wasm32"))))]
	let inner = ();
	(WakeSignaler(inner.clone()), WakeWaiter(inner))
}

impl WakeWaiter {
	/// Waits until another cloned instance of [`WakeSignaler`] has been dropped.
	/// If any cloned instance of [`WakeSignaler`] is dropped then this wait stops waiting.
	#[inline]
	pub(crate) fn wait(&self) {
		#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
		{
			let (lock, cv) = &*self.0;
			let mut signaled = lock.lock().unwrap();
			while !*signaled {
				signaled = cv.wait(signaled).unwrap();
			}
		}
		// no-op on no_std / wasm: futures are ticked synchronously before this point.
	}
}
impl Drop for WakeSignaler {
	#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
	#[inline]
	fn drop(&mut self) {
		let (lock, cv) = &*self.0;
		let mut signaled = lock.lock().unwrap();
		*signaled = true;
		cv.notify_one();
	}

	#[cfg(not(all(feature = "std", not(target_arch = "wasm32"))))]
	#[inline]
	fn drop(&mut self) {}
}
