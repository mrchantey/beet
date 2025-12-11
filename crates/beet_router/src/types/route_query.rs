use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

pub trait ActionExchangePair {
	fn get_action(&self) -> Entity;
	fn get_exchange(&self) -> Entity;
}
impl<'w, 't, T> ActionExchangePair for On<'w, 't, T>
where
	T: ActionEvent,
{
	fn get_action(&self) -> Entity { self.action() }
	fn get_exchange(&self) -> Entity { self.agent() }
}

impl ActionExchangePair for EndpointContext {
	fn get_action(&self) -> Entity { self.action_id() }
	fn get_exchange(&self) -> Entity { self.exchange_id() }
}
impl ActionExchangePair for MiddlewareContext {
	fn get_action(&self) -> Entity { self.action() }
	fn get_exchange(&self) -> Entity { self.exchange() }
}


#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	requests: Query<'w, 's, &'static RequestMeta>,
	// agents: Query<'w, 's, &'static mut ExchangeContext>,
	parents: Query<'w, 's, &'static ChildOf>,
	partials: Query<'w, 's, &'static RoutePartial>,
}

impl RouteQuery<'_, '_> {
	pub fn path(&self, ev: &impl ActionExchangePair) -> Result<RoutePath> {
		self.requests.get(ev.get_exchange())?.path().xok()
	}
	pub fn method(&self, ev: &impl ActionExchangePair) -> Result<HttpMethod> {
		self.requests.get(ev.get_exchange())?.method().xok()
	}

	pub fn dyn_segment(
		&mut self,
		ev: &impl ActionExchangePair,
		key: &str,
	) -> Result<String> {
		self.route_match(ev)?
			.dyn_map
			.get(key)
			.map(|key| key.clone())
			.ok_or_else(|| bevyhow!("key not found: {}", key))
	}

	pub fn route_pattern(&self, action: Entity) -> Result<RoutePattern> {
		self.parents
			// get every PathFilter in ancestors
			.iter_ancestors_inclusive(action)
			.filter_map(|entity| self.partials.get(entity).ok())
			.collect::<Vec<_>>()
			.into_iter()
			.cloned()
			// reverse to start from the root
			.rev()
			// extract the segments
			.flat_map(|filter| filter.segments)
			.collect::<Vec<_>>()
			.xmap(RoutePattern::from_segments)
	}

	pub fn route_match(
		&self,
		ev: &impl ActionExchangePair,
	) -> Result<RouteMatch> {
		let path = self.path(ev)?;
		let pattern = self.route_pattern(ev.get_action())?;
		// println!("matching path '{}' against pattern '{}'", path, pattern);

		pattern.parse_path(&path)?.xok()
	}
}
