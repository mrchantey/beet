use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use bevy::reflect::Typed;


// temp impl for matching an action with the exchange agent( request/response entity)
// when we get many-many relations this should be easier
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


// temp, this is getting messy
pub struct ActionExchange {
	pub action: Entity,
	pub exchange: Entity,
}
impl ActionExchangePair for ActionExchange {
	fn get_action(&self) -> Entity { self.action }
	fn get_exchange(&self) -> Entity { self.exchange }
}


#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	pub requests: Query<'w, 's, &'static RequestMeta>,
	// agents: Query<'w, 's, &'static mut ExchangeContext>,
	pub parents: Query<'w, 's, &'static ChildOf>,
	pub path_partials: Query<'w, 's, &'static PathPartial>,
	pub params_partials: Query<'w, 's, &'static ParamsPartial>,
}

impl RouteQuery<'_, '_> {
	pub fn path(&self, ev: &impl ActionExchangePair) -> Result<RoutePath> {
		self.requests.get(ev.get_exchange())?.path().xok()
	}
	pub fn method(&self, ev: &impl ActionExchangePair) -> Result<HttpMethod> {
		self.requests.get(ev.get_exchange())?.method().xok()
	}

	pub fn params<T: 'static + Send + Sync + FromReflect + Typed>(
		&self,
		exchange: Entity,
	) -> Result<T> {
		self.requests.get(exchange)?.params().parse::<T>()
	}

	pub fn path_match(
		&self,
		ev: &impl ActionExchangePair,
	) -> Result<PathMatch> {
		let path = self.path(ev)?;
		let pattern = PathPattern::collect(ev.get_action(), &self)?;
		pattern.parse_path(&path)?.xok()
	}

	pub fn dyn_segment(
		&mut self,
		ev: &impl ActionExchangePair,
		key: &str,
	) -> Result<String> {
		self.path_match(ev)?
			.dyn_map
			.get(key)
			.map(|key| key.clone())
			.ok_or_else(|| bevyhow!("key not found: {}", key))
	}
}
