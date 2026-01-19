use crate::prelude::*;
use beet_core::prelude::*;

/// The function called for each request, spawning
/// or retrieving the entity upon which a request will be inserted,
/// and a response will be retrieved, see [`handle_request`]
/// see [`default_handler`] for the default implementation.
pub fn spawn_exchange<F, B>(func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Fn() -> B,
	B: Bundle,
{
	OnSpawn::observe(
		move |ev: On<ExchangeStart>, mut commands: Commands| -> Result {
			let spawner_entity = ev.event_target();
			let ExchangeContext { request, end } = ev.take()?;
			let mut entity = commands.spawn((
				ChildOf(spawner_entity),
				OnSpawn::observe(end_on_insert_response),
				func(),
				end,
			));
			// insert request after spawner, giving it a
			// chance to insert observers
			entity.insert(request);

			Ok(())
		},
	)
}

/// End the exchange when a Response is inserted
// this would be an exclusive observer but thats not yet supported
fn end_on_insert_response(
	ev: On<Insert, Response>,
	mut commands: Commands,
) -> Result {
	let exchange_entity = ev.event_target();
	commands
		.entity(exchange_entity)
		.queue(take_and_send_response);
	Ok(())
}

fn take_and_send_response(mut entity: EntityWorldMut) -> Result {
	let response = entity
		.take::<Response>()
		.unwrap_or_else(|| Response::not_found());
	entity
		.get::<ExchangeEnd>()
		.ok_or_else(|| bevyhow!("ExchangeEnd not found"))?
		.send(response)?;
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	// #[beet_core::test(timeout_ms = 500)]
	async fn works() {
		PrettyTracing::default().init();
		let mut world = World::new();
		let mut entity = world.spawn(spawn_exchange(|| {
			OnSpawn::observe(
				|ev: On<Insert, Request>,
				 mut commands: Commands,
				 requests: Query<&Request>| {
					commands.entity(ev.event_target()).insert(
						requests.get(ev.event_target()).unwrap().mirror_parts(),
					);
				},
			)
		}));
		let res = Request::get("/foo").exchange(&mut entity).await;
		res.status().xpect_eq(StatusCode::Ok);
		res.path_string().xpect_eq("/foo");
	}
}
