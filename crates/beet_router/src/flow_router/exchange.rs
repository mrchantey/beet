


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;


	fn setup(request: Request) -> Option<StatusCode> {
		let mut world = FlowRouterPlugin.into_world();
		let exchange = world.spawn(request).id();

		let store = Store::default();

		world
			.spawn_empty()
			.observe_any(
				|mut ev: On<GetOutcome>,
				 mut commands: Commands,
				 requests: Query<&Request>|
				 -> Result {
					let req = requests.get(ev.agent())?;
					let status = if req.body.is_some() {
						StatusCode::OK
					} else {
						StatusCode::BAD_REQUEST
					};
					commands
						.entity(ev.agent())
						.insert(Response::from_status(status));
					ev.trigger_next(Outcome::Pass);
					Ok(())
				},
			)
			.observe_any(
				move |ev: On<Outcome>, responses: Query<&Response>| -> Result {
					let response = responses.get(ev.agent())?;
					store.set(Some(response.status()));
					Ok(())
				},
			)
			.trigger_target(GetOutcome.with_agent(exchange))
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
