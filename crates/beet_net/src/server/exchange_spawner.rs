use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;


// Placeholder, will probs be replaced by bevy Template system
pub trait BundleFunc: 'static + Send + Sync + Clone {
	fn bundle_func(self) -> impl Bundle;
}
impl<F, T> BundleFunc for F
where
	F: 'static + Send + Sync + Clone + FnOnce() -> T,
	T: Bundle,
{
	fn bundle_func(self) -> impl Bundle { self() }
}

/// The function called for each request,
/// see [`default_handler`] for the default implementation.
#[derive(Clone, Deref, Component)]
pub struct ExchangeSpawner {
	func: SpawnFunc,
}

/// We accept an &mut World to allow for entity pooling
type SpawnFunc = Arc<Box<dyn 'static + Send + Sync + Fn(&mut World) -> Entity>>;

impl ExchangeSpawner {
	pub fn new<Func>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Fn(&mut World) -> Entity,
	{
		Self {
			func: Arc::new(Box::new(func)),
		}
	}
	pub fn new_bundle(func: impl BundleFunc) -> Self {
		Self::new(move |world: &mut World| {
			world.spawn(func.clone().bundle_func()).id()
		})
	}

	pub fn new_handler(
		func: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(EntityWorldMut, Request) -> Response,
	) -> Self {
		Self::new_bundle(move || {
			let func = func.clone();
			OnSpawn::observe(
				move |ev: On<Insert, Request>, mut commands: Commands| {
					let func = func.clone();
					let entity = ev.event_target();
					commands.queue(move |world: &mut World| -> Result {
						let req = world
							.entity_mut(entity)
							.take::<Request>()
							.ok_or_else(|| {
								bevyhow!(
									"Exchange entity missing Request component"
								)
							})?;
						let response = func(world.entity_mut(entity), req);
						world.entity_mut(entity).insert(response);
						Ok(())
					});
				},
			)
		})
	}

	pub fn new_handler_async<Fut>(
		func: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(AsyncEntity, Request) -> Fut,
	) -> Self
	where
		Fut: Send + Future<Output = Response>,
	{
		Self::new_bundle(move || {
			let func = func.clone();
			OnSpawn::observe(
				move |ev: On<Insert, Request>, mut commands: AsyncCommands| {
					let func = func.clone();
					let entity = ev.event_target();
					commands.run(async move |world: AsyncWorld| -> Result {
						let entity = world.entity(entity);
						let req = world
							.entity(entity.id())
							.take::<Request>()
							.await
							.ok_or_else(|| {
								bevyhow!(
									"Exchange entity missing Request component"
								)
							})?;
						let response = func(entity.clone(), req).await;
						entity.insert(response);
						Ok(())
					});
				},
			)
		})
	}

	pub(super) fn spawn(&self, world: &mut World) -> Entity {
		(self.func)(world)
	}

	pub fn mirror() -> Self {
		Self::new_handler(|_, req| {
			Response::new(
				ResponseParts {
					parts: req.parts().parts().clone(),
					status: StatusCode::OK,
				},
				req.body,
			)
		})
	}
}


impl Default for ExchangeSpawner {
	fn default() -> Self { Self::mirror() }
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
	async fn works() {
		parse(ExchangeSpawner::new_handler(|_, _| {
			StatusCode::IM_A_TEAPOT.into()
		}))
		.await
		.status()
		.xpect_eq(StatusCode::IM_A_TEAPOT);
	}
}
