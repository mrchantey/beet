use crate::prelude::*;
use beet_core::prelude::*;


/// The default route handler:
/// - Creates a child of the server inserting the [`Request`] component
/// - Adds a one-shot observer for [`On<Insert, Response>`],
///   then takes the response and despawns the entity.
/// the default handler adds about 100us to a request that
/// doesnt involve mutating the world or running systems: (40us vs 140us)
pub async fn default_handler(
	entity: AsyncEntity,
	request: Request,
) -> Response {
	let id = entity.id();
	let (send, recv) = async_channel::bounded(1);
	let exchange_entity = entity
		.world()
		.with_then(move |world| {
			world
				.spawn(ExchangeOf(id))
				// add observer before inserting request to handle immediate response
				.observe(
					move |ev: On<Insert, Response>, mut commands: Commands| {
						let exchange = ev.event_target();
						let send = send.clone();
						commands.queue(move |world: &mut World| {
							let response = world
								.entity_mut(exchange)
								.take::<Response>()
								.unwrap_or_else(|| Response::not_found());
							send.try_send(response)
								.expect("unreachable, we await recv");
						});
					},
				)
				.insert(request)
				.id()
		})
		.await;

	let response = recv.recv().await.unwrap_or_else(|_| {
		error!("Sender was dropped, was the world dropped?");
		Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	});

	// cleanup exchange entity after response is received
	entity
		.world()
		.with_then(move |world| {
			if let Ok(exchange) = world.get_entity_mut(exchange_entity) {
				exchange.despawn();
			}
		})
		.await;

	response
}


pub fn exchange_meta(
	ev: On<Insert, Response>,
	mut servers: Query<&mut ServerStatus>,
	exchange: Query<(&RequestMeta, &Response, &ExchangeOf)>,
) -> Result {
	let entity = ev.event_target();
	let Ok((meta, response, exchange_of)) = exchange.get(entity) else {
		// ignore if no match, probably a test
		return Ok(());
	};
	let status = response.status();
	let duration = meta.started().elapsed();
	let path = meta.path();
	let method = meta.method();

	let mut stats = servers.get_mut(exchange_of.get())?;

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
}


#[cfg(test)]
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let server = HttpServer::new_test();
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
