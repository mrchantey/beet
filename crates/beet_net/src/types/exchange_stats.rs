//! Exchange statistics tracking and logging.
//!
//! This module provides [`ExchangeStats`] for tracking request counts
//! and the [`exchange_stats`] observer for logging exchange completion.
// the wire-event imports (`ExchangeEnd` etc.) are only used by the observer,
// which needs the `action` feature.
#[cfg(feature = "action")]
use super::*;
use beet_core::prelude::*;

/// Observer that logs each exchanged request and bumps the server's request
/// counter, registered by [`ServerPlugin`](crate::prelude::ServerPlugin).
///
/// Logs a single concise line per request — method, path, status, duration, and
/// running request index — at `info`. The method/path/status/timing ride on the
/// [`ExchangeEnd`] event fired by
/// [`exchange`](crate::prelude::AsyncExchangeExt::exchange).
///
/// `action`-gated (its only non-no_std dep): it reads the [`ExchangeEnd`] event.
/// The [`ExchangeStats`] counter it bumps is itself no_std (it backs the no_std
/// [`HttpServer`] requirement).
#[cfg(feature = "action")]
pub fn exchange_stats(
	ev: On<ExchangeEnd>,
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
/// Add this to server entities to track the number of requests processed.
/// The [`exchange_stats`] observer will automatically update these stats
/// when [`ExchangeEnd`] events are triggered.
#[derive(Default, Component)]
pub struct ExchangeStats {
	request_count: u128,
}

impl ExchangeStats {
	/// Returns the total number of requests processed.
	pub fn request_count(&self) -> u128 { self.request_count }

	/// Increments the request counter.
	// only the `action`-gated logging observer bumps it today; a backend may too.
	#[cfg_attr(not(feature = "action"), allow(dead_code))]
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}

#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		let mut world = AsyncPlugin::world();
		world.add_observer(exchange_stats);

		let entity = world
			.spawn((
				ExchangeStats::default(),
				DispatchExchange(exchange_handler(|req| req.mirror_parts())),
			))
			.id();

		world
			.entity_mut(entity)
			.exchange(Request::get("/mirror"))
			.await
			.into_result()
			.await
			.xpect_ok();

		world
			.query_once::<&ExchangeStats>()
			.iter()
			.next()
			.unwrap()
			.request_count()
			.xpect_eq(1);
	}
}
