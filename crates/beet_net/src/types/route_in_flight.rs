//! The per-route in-flight count: how many requests hold a route mid-dispatch,
//! so a teardown that replaces the route (a live reload's swap) can keep it
//! alive until the last one answers rather than cutting the call short.
use alloc::sync::Arc;
use beet_core::prelude::*;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

/// The number of requests dispatched to this route and not yet answered.
///
/// Required by every [`PathPattern`](crate::prelude::PathPattern), so each
/// route in a tree carries one. The dispatch takes an [`InFlightGuard`] in the
/// same world access that resolves the route, holding it until the handler
/// answers; a route replaced while held is retired instead of despawned, and
/// despawns once [`is_idle`](Self::is_idle).
#[derive(Default, Component)]
// a clone of a route is a fresh route, so it starts idle through the require
#[component(clone_behavior = Ignore)]
pub struct RouteInFlight(Arc<AtomicUsize>);

impl RouteInFlight {
	/// The number of requests currently held on this route.
	pub fn count(&self) -> usize { self.0.load(Ordering::SeqCst) }

	/// Whether no request holds this route.
	pub fn is_idle(&self) -> bool { self.count() == 0 }

	/// Hold the route in flight until the returned guard drops.
	pub fn begin(&self) -> InFlightGuard {
		self.0.fetch_add(1, Ordering::SeqCst);
		InFlightGuard(self.0.clone())
	}
}

/// Holds one request in flight on a [`RouteInFlight`], released on drop, so a
/// dispatch cancelled mid-call (its connection torn down) never leaves a
/// phantom count behind.
#[must_use = "dropping the guard releases the route"]
pub struct InFlightGuard(Arc<AtomicUsize>);

impl Drop for InFlightGuard {
	fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); }
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn counts_until_the_guard_drops() {
		let in_flight = RouteInFlight::default();
		in_flight.is_idle().xpect_true();
		let first = in_flight.begin();
		let second = in_flight.begin();
		in_flight.count().xpect_eq(2);
		drop(first);
		in_flight.is_idle().xpect_false();
		drop(second);
		in_flight.is_idle().xpect_true();
	}
}
