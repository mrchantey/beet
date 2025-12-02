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
	agents: Query<'w, 's, &'static mut ExchangeContext>,
	parents: Query<'w, 's, &'static ChildOf>,
}

impl RouteQuery<'_, '_> {
	/// Get the [`RouteContex`] for this events `agent` / `action` pair.
	/// - If none exits, the [`RouteContext`] from the nearest ancestor will
	/// be cloned.
	/// - If none exists in ancestors, one will be created from the
	///   [`Request`] and inserted.
	///
	/// Note that the cx may not be valid, for instance if a
	/// [`PathFilter`] exists on the [`action`] and it failed to be consumed entirely.
	/// Its up to the user to make actions expecting a valid cx unreachable
	/// by using a [`check_path_filter`] or similar.
	///
	/// ## Errors
	/// Errors if no [`RouteContex`] in ancestors and the [`Request`]
	/// could not be found on the `agent` to create one.
	pub fn with_cx<O>(
		&mut self,
		ev: &impl ActionExchangePair,
		mut func: impl FnMut(&mut RouteContext) -> O,
	) -> Result<O> {
		let mut cx = self.agents.get_mut(ev.get_exchange())?;
		let cx_map = cx.route_context_map_mut();
		// 1. check if it exists
		if let Some(cx) = cx_map.get_mut(&ev.get_action()) {
			return Ok(func(cx));
		}
		// 2. check ancestors and clone
		for entity in self.parents.iter_ancestors(ev.get_action()) {
			if let Some(parent_cx) = cx_map.get(&entity) {
				let cx = parent_cx.clone();
				cx_map.insert(ev.get_action(), cx);
				let cx = cx_map.get_mut(&ev.get_action()).unwrap();
				return Ok(func(cx));
			}
		}
		// 3. create from request
		let request = self.requests.get(ev.get_exchange())?;

		let cx = RouteContext::new(request.path());
		cx_map.insert(ev.get_action(), cx);
		let cx = cx_map.get_mut(&ev.get_action()).unwrap();
		return Ok(func(cx));
	}

	pub fn method(&self, ev: &impl ActionExchangePair) -> Result<HttpMethod> {
		self.requests.get(ev.get_exchange())?.method().xok()
	}

	pub fn dyn_segment(
		&mut self,
		ev: &impl ActionExchangePair,
		key: &str,
	) -> Result<String> {
		self.with_cx(ev, |cx| cx.dyn_segment(key).cloned())?
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::exports::Itertools;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn route_cx_propagates() {
		let mut world = RouterPlugin::world();
		let (send, _recv) = async_channel::bounded(1);
		let agent = world
			.spawn((Request::get("/foo"), ExchangeContext::new(send)))
			.id();
		let store = Store::default();

		// spawn the parent, set its context, then check the child
		world
			.spawn((
				OnSpawn::observe(
					move |mut ev: On<GetOutcome>,
					      mut query: RouteQuery,
					      children: Query<&Children>| {
						query
							.with_cx(&ev, move |cx| {
								let _ = std::mem::replace(
									cx,
									RouteContext::new("/bar"),
								);
							})
							.unwrap();
						ev.trigger_action_with_cx(
							children.get(ev.action()).unwrap()[0],
							GetOutcome,
						);
					},
				),
				children![OnSpawn::observe(
					move |mut ev: On<GetOutcome>, mut query: RouteQuery| {
						// println!("cx: {:?}", cx.path());
						query
							.with_cx(&ev, move |cx| {
								store.set(cx.path().clone().iter().join("/"));
							})
							.unwrap();
						ev.trigger_with_cx(Outcome::Pass);
					}
				)],
			))
			.trigger_target(GetOutcome.with_agent(agent))
			.flush();

		store.get().xpect_eq("bar");
	}
}
