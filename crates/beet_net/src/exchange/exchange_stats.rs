//! Exchange statistics tracking and logging.
//!
//! This module provides [`ExchangeStats`] for tracking request counts and
//! the [`exchange_stats`] observer for logging exchange completion.
use crate::prelude::*;
use beet_core::prelude::*;



/// Observer that logs exchange completion and updates statistics.
///
/// This observer should be added to the world to enable request logging.
/// It logs the request path, method, status, duration, and request index.
///
/// # Example
///
/// ```ignore
/// world.add_observer(exchange_stats);
/// ```
pub fn exchange_stats(
	ev: On<ExchangeEnd>,
	mut servers: AncestorQuery<&mut ExchangeStats>,
	exchange: Query<&RequestMeta>,
) -> Result {
	let entity = ev.event_target();

	let mut stats = Vec::new();

	// only available for spawn type exchanges
	if let Ok(meta) = exchange.get(entity) {
		stats.push(format!("path:\t{}", meta.path_string()));
		stats.push(format!("method:\t{}", meta.method()));
	};

	stats.push(format!("status:\t{}", ev.status));

	stats.push(format!(
		"duration:\t{}",
		time_ext::pretty_print_duration(ev.start_time.elapsed())
	));

	if let Ok(mut server) = servers.get_mut(entity) {
		server.increment_requests();
		stats.push(format!("index:\t{}", server.request_count()));
	}

	bevy::log::info!("Request Complete:\n{}", stats.join("\n"));
	Ok(())
}



/// Component for tracking exchange statistics on a server entity.
///
/// Add this to server entities to track the number of requests processed.
/// The [`exchange_stats`] observer will automatically update these stats.
#[derive(Default, Component)]
pub struct ExchangeStats {
	request_count: u128,
}

impl ExchangeStats {
	/// Returns the total number of requests processed.
	pub fn request_count(&self) -> u128 { self.request_count }

	/// Increments the request counter.
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		let mut world = AsyncPlugin::world();
		world.add_observer(exchange_stats);
		world
			.spawn((
				ExchangeStats::default(),
				spawn_exchange(|| {
					OnSpawn::observe(
						|ev: On<Insert, Request>,
						 mut commands: Commands,
						 requests: Query<&Request>| {
							commands.entity(ev.event_target()).insert(
								requests
									.get(ev.event_target())
									.unwrap()
									.mirror_parts(),
							);
						},
					)
				}),
			))
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
