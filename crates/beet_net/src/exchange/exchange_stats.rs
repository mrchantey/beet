use crate::prelude::*;
use beet_core::prelude::*;



/// Update server stats if available
pub fn exchange_stats(
	ev: On<ExchangeEnd>,
	mut servers: Query<&mut ExchangeStats>,
	exchange: Query<(&RequestMeta, &Response, &ExchangeOf)>,
) -> Result {
	let entity = ev.event_target();
	let Ok((meta, response, exchange_of)) = exchange.get(entity) else {
		return Ok(());
	};
	let status = response.status();
	let duration = meta.started().elapsed();
	let path = meta.path_string();
	let method = meta.method();

	let Ok(mut stats) = servers.get_mut(exchange_of.get()) else {
		return Ok(());
	};

	bevy::log::info!(
		"
Request Complete
  path:     {}
  method:   {}
  duration: {}
  status:   {}
  index:    {}
",
		path,
		method,
		time_ext::pretty_print_duration(duration),
		status,
		stats.request_count()
	);
	stats.increment_requests();
	Ok(())
	// todo!("update to new flow");
}



#[derive(Default, Component)]
pub struct ExchangeStats {
	request_count: u128,
}
impl ExchangeStats {
	pub fn request_count(&self) -> u128 { self.request_count }
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}
