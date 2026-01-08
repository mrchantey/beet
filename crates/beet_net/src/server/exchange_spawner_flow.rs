use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

impl ExchangeSpawner {
	/// Create a new ExchangeSpawner compatible with control flow structures.
	/// 1. Upon a [`Request`] insert, [`GetOutcome`] will be triggered on the same entity
	/// 2. Upon an [`Outcome`] a response will be inserted if none has been inserted already:
	/// 	- [`Outcome::Pass`] -> [`StatusCode::OK`]
	/// 	- [`Outcome::Fail`] -> [`StatusCode::INTERNAL_SERVER_ERROR`]
	pub fn new_flow(func: impl BundleFunc) -> Self {
		Self::new_bundle(move || {
			(
				OnSpawn::observe(
					|ev: On<Insert, Request>, mut commands: Commands| {
						commands
							.entity(ev.event_target())
							.trigger_target(GetOutcome);
					},
				),
				OnSpawn::observe(
					|ev: On<Outcome>,
					 mut commands: Commands,
					 has_response: Query<(), With<ResponseMarker>>| {
						if !has_response.contains(ev.target()) {
							let status = match ev.event() {
								Outcome::Pass => StatusCode::OK,
								Outcome::Fail => {
									StatusCode::INTERNAL_SERVER_ERROR
								}
							};
							commands
								.entity(ev.target())
								.insert(Response::from_status(status));
						}
					},
				),
				func.bundle_func(),
			)
		})
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	async fn parse(bundle: impl Bundle) -> Response {
		App::new()
			.add_plugins((MinimalPlugins, ServerPlugin))
			.world_mut()
			.spawn(bundle)
			.oneshot(Request::get("foo"))
			.await
	}

	#[sweet::test]
	async fn flow_inserts_response() {
		use beet_flow::prelude::*;
		parse(ExchangeSpawner::new_flow(|| {
			OnSpawn::observe(|ev: On<GetOutcome>, mut commands: Commands| {
				commands
					.entity(ev.target())
					.insert(Response::from_status(StatusCode::IM_A_TEAPOT));
			})
		}))
		.await
		.status()
		.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[sweet::test]
	async fn flow_outcome_pass() {
		use beet_flow::prelude::*;
		parse(ExchangeSpawner::new_flow(|| EndWith(Outcome::Pass)))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[sweet::test]
	async fn flow_outcome_fail() {
		use beet_flow::prelude::*;
		parse(ExchangeSpawner::new_flow(|| EndWith(Outcome::Fail)))
			.await
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}
}
