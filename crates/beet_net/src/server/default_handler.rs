use crate::prelude::*;
use beet_core::prelude::*;


/// The default route handler:
/// - Creates a child of the server inserting the [`Request`] component
/// - Adds a one-shot observer for [`On<Insert, Response>`],
///   then takes the response and despawns the entity.
/// the default handler, adds about 100us to a request that
/// doesnt involve the world at all: (40us vs 140us)
pub async fn default_handler(
	entity: AsyncEntity,
	request: Request,
) -> Response {
	let (send, recv) = async_channel::bounded(1);
	entity
		.insert(children![(
			OnSpawn::observe(
				move |ev: On<Insert, Response>, mut commands: Commands| {
					let entity = ev.event_target();
					let send = send.clone();
					commands.queue(move |world: &mut World| {
						let response = world
							.entity_mut(entity)
							.take::<Response>()
							.unwrap_or_else(|| Response::not_found());
						world.entity_mut(entity).despawn();
						send.try_send(response)
							.expect("unreachable, we await recv");
					});
				}
			),
			OnSpawn::new(move |entity| {
				// slighly defer inserting request so the observer can mount
				entity.insert(request);
			}) // req,
		)])
		.await;

	recv.recv().await.unwrap_or_else(|_| {
		error!("Sender was dropped, was the world dropped?");
		Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	})
}


pub fn exchange_meta(
	ev: On<Insert, Response>,
	parents: Query<&ChildOf>,
	mut servers: Query<&mut ServerStatus>,
	exchange: Query<(&RequestMeta, &Response)>,
) -> Result {
	let entity = ev.event_target();
	let (meta, response) = exchange.get(entity)?;
	let status = response.status();
	let duration = meta.started().elapsed();
	let path = meta.path();
	let method = meta.method();

	bevy::log::info!(
		"
	Request Complete
	  path: {}
	  method: {}
	  duration: {}
	  status: {}
								",
		method,
		path,
		time_ext::pretty_print_duration(duration),
		status
	);

	if let Ok(parent) = parents.get(entity)
		&& let Ok(mut stats) = servers.get_mut(parent.parent())
	{
		stats.increment_requests();
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let server = Server::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((
					MinimalPlugins,
					ServerPlugin::with_server(server),
				))
				.add_observer(
					|ev: On<Insert, Request>, mut commands: Commands| {
						commands
							.entity(ev.event_target())
							.insert(Response::ok().with_body("hello"));
					},
				)
				.run();
		});
		for _ in 0..10 {
			Request::post(&url)
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.text()
				.await
				.unwrap()
				.xpect_eq("hello");
		}
	}
}
