use beet_core::prelude::*;
use beet_net::prelude::*;


#[derive(Debug, Clone, Deref, DerefMut, ActionEvent)]
pub struct RunExchange {
	pub exchange: Entity,
}
impl RunExchange {
	pub fn new(exchange: Entity) -> Self { Self { exchange } }
}

#[derive(Debug, Clone, Deref, DerefMut, ActionEvent)]
pub struct EndExchange {
	pub exchange: Entity,
}
impl EndExchange {
	pub fn new(exchange: Entity) -> Self { Self { exchange } }

	pub fn trigger(mut commands: Commands, ev: &On<RunExchange>) {
		commands
			.entity(ev.event_target())
			.trigger_target(Self::new(ev.exchange));
	}
}

/// A [`SystemParam`] used for convenien
#[derive(SystemParam)]
pub struct ExchangeQuery<'w, 's, D = (), F = ()>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
	pub parents: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub query: Query<'w, 's, D, F>,
	pub requests: Query<'w, 's, &'static mut Request>,
	pub responses: Query<'w, 's, &'static mut Response>,
}
impl<'w, 's, D, F> ExchangeQuery<'w, 's, D, F>
where
	D: 'static + QueryData,
	F: 'static + QueryFilter,
{
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;


	fn setup(request: Request) -> Option<StatusCode> {
		let mut world = FlowRouterPlugin.into_world();
		let exchange = world.spawn(request).id();

		let store = Store::default();

		world
			.spawn_empty()
			.observe_any(
				|ev: On<RunExchange>,
				 mut commands: Commands,
				 requests: Query<&Request>|
				 -> Result {
					let req = requests.get(ev.exchange)?;
					let status = if req.body.is_some() {
						StatusCode::OK
					} else {
						StatusCode::BAD_REQUEST
					};
					commands
						.entity(ev.exchange)
						.insert(Response::from_status(status));
					EndExchange::trigger(commands, &ev);
					Ok(())
				},
			)
			.observe_any(
				move |ev: On<EndExchange>,
				      responses: Query<&Response>|
				      -> Result {
					let response = responses.get(ev.exchange)?;
					store.set(Some(response.status()));
					Ok(())
				},
			)
			.trigger_target(RunExchange::new(exchange))
			.flush();

		store.get().clone()
	}

	#[test]
	fn works() {
		setup(Request::get("foo")).xpect_eq(Some(StatusCode::BAD_REQUEST));
		setup(Request::get("foo").with_body([0, 1, 2]))
			.xpect_eq(Some(StatusCode::OK));
	}
}
