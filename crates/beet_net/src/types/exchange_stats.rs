//! Exchange statistics tracking and logging.
//!
//! This module provides [`ExchangeStats`] for tracking request counts
//! and the [`exchange_stats`] observer for logging exchange completion.
// the wire-event imports (`EndExchange` etc.) are only used by the observer,
// which needs the `action` feature.
#[cfg(feature = "action")]
use super::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

/// Observer that logs each exchanged request and bumps the server's request
/// counter, registered by [`ServerPlugin`](crate::prelude::ServerPlugin).
///
/// Logs a single concise line per request — method, path, status, duration, and
/// running request index — at `info`. The method/path/status/timing ride on the
/// [`EndExchange`] event fired by
/// [`exchange`](crate::prelude::AsyncExchangeExt::exchange).
///
/// `action`-gated (its only non-no_std dep): it reads the [`EndExchange`] event.
/// The [`ExchangeStats`] counter it bumps is itself no_std (it backs the no_std
/// [`HttpServer`] requirement).
#[cfg(feature = "action")]
pub(crate) fn exchange_stats(
	ev: On<EndExchange>,
	mut servers: AncestorQuery<&mut ExchangeStats>,
) -> Result {
	let entity = ev.event_target();

	let index = servers
		.get_mut(entity)
		.map(|mut server| server.increment_requests().request_count())
		.ok();

	info!(
		"{} {} -> {} in {}{}",
		ev.method,
		ev.path,
		ev.status,
		time_ext::pretty_print_duration(ev.start_time.elapsed()),
		index.map(|i| format!(" (#{i})")).unwrap_or_default(),
	);
	Ok(())
}

/// Component for tracking exchange statistics on a server entity.
///
/// Add this to server entities to track the number of requests processed and
/// the number currently being served. The [`exchange_stats`] observer bumps the
/// processed count on [`EndExchange`]; the dispatch itself
/// ([`exchange`](crate::prelude::AsyncExchangeExt::exchange)) brackets each
/// request in [`in_flight`](Self::in_flight), so a teardown (a live reload's
/// route respawn) can wait for the requests it would otherwise cut off.
#[derive(Default, Component)]
pub struct ExchangeStats {
	request_count: u128,
	/// Shared with each in-flight request's [`InFlightGuard`], which releases
	/// it on drop, so a request cancelled mid-dispatch (its connection torn
	/// down with the server) never leaves a phantom count behind.
	in_flight: Arc<AtomicUsize>,
}

impl ExchangeStats {
	/// A counter seeded with `request_count` already processed, for tests and
	/// diagnostics that need a non-zero starting count.
	pub fn new(request_count: u128) -> Self {
		Self {
			request_count,
			in_flight: default(),
		}
	}

	/// Returns the total number of requests processed.
	pub fn request_count(&self) -> u128 { self.request_count }

	/// The number of requests dispatched and not yet answered.
	pub fn in_flight(&self) -> usize { self.in_flight.load(Ordering::SeqCst) }

	/// Increments the request counter.
	// only the `action`-gated logging observer bumps it today; a backend may too.
	#[cfg_attr(not(feature = "action"), allow(dead_code))]
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}

	/// Count a request as in flight until the returned guard drops.
	pub(super) fn begin_request(&self) -> InFlightGuard {
		self.in_flight.fetch_add(1, Ordering::SeqCst);
		InFlightGuard(self.in_flight.clone())
	}
}

/// Holds one in-flight request on an [`ExchangeStats`], released on drop.
pub(super) struct InFlightGuard(Arc<AtomicUsize>);

impl Drop for InFlightGuard {
	fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); }
}

#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		let mut world = AsyncPlugin::world();
		world.add_observer(exchange_stats);

		let entity = world
			.spawn((
				ExchangeStats::default(),
				exchange_ext::handler(|req| req.mirror_parts()),
			))
			.id();

		world
			.entity_mut(entity)
			.exchange(Request::get("/mirror"))
			.await
			.into_result()
			.await
			.xpect_ok();

		let stats = world.query_once::<&ExchangeStats>()[0];
		stats.request_count().xpect_eq(1);
		// answered, so nothing is left in flight
		stats.in_flight().xpect_eq(0);
	}

	/// A request counts as in flight from dispatch until its handler answers:
	/// the handler here observes the count it is part of.
	#[beet_core::test]
	async fn counts_requests_in_flight() {
		let mut world = AsyncPlugin::world();
		let seen = Store::new(0);
		let recorder = seen.clone();
		let entity = world
			.spawn((
				ExchangeStats::default(),
				Action::<Request, Response>::new_async(
					move |cx: ActionContext<Request>| {
						let recorder = recorder.clone();
						let caller = cx.caller.clone();
						async move {
							let in_flight = caller
								.get::<ExchangeStats, usize>(|stats| {
									stats.in_flight()
								})
								.await?;
							recorder.set(in_flight);
							Result::<Response>::Ok(Response::ok())
						}
					},
				),
			))
			.id();
		world.entity_mut(entity).exchange(Request::get("/")).await;
		seen.get().xpect_eq(1);
		world.query_once::<&ExchangeStats>()[0]
			.in_flight()
			.xpect_eq(0);
	}
}
