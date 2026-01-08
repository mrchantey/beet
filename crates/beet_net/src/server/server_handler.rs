use std::pin::Pin;
use std::sync::Arc;

use crate::prelude::*;
use beet_core::prelude::*;


/// The function called for each request,
/// see [`default_handler`] for the default implementation.
#[derive(Clone, Deref, Component)]
pub struct ServerHandler {
	handler: HandlerFn,
}

impl ServerHandler {
	pub fn new<F, Fut>(func: F) -> Self
	where
		F: 'static + Send + Sync + Clone + FnOnce(AsyncEntity, Request) -> Fut,
		Fut: Send + Future<Output = Response>,
	{
		Self {
			handler: Arc::new(Box::new(move |world, request| {
				let func = func.clone();
				Box::pin(async move { func.clone()(world, request).await })
			})),
		}
	}
	pub fn mirror() -> Self {
		Self::new(|_, req| async {
			Response::new(
				ResponseParts {
					parts: req.parts().parts().clone(),
					status: StatusCode::OK,
				},
				req.body,
			)
		})
	}

	pub fn handler(&self) -> HandlerFn { self.handler.clone() }
}

impl Default for ServerHandler {
	fn default() -> Self { Self::new(default_handler) }
}

pub(super) type HandlerFn = Arc<
	Box<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncEntity,
				Request,
			) -> Pin<Box<dyn Send + Future<Output = Response>>>,
	>,
>;



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
