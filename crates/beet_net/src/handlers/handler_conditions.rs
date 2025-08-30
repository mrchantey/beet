use crate::prelude::*;
use bevy::prelude::*;
use std::pin::Pin;
use std::sync::Arc;

/// A collection of predicates that must pass for a [`RouteHandler`] to run.
#[derive(Default, Component, Clone)]
pub struct HandlerConditions(Vec<Arc<Predicate>>);

type Predicate = dyn 'static
	+ Send
	+ Sync
	+ Fn(World, Entity) -> Pin<Box<dyn Future<Output = (World, bool)> + Send>>;


impl HandlerConditions {
	/// Runs only if:
	/// 1. There is a [`Request`]
	/// 2. There is no [`Response`]
	pub fn fallback() -> Self {
		Self::default().system(
			|req: Option<Res<Request>>, res: Option<Res<Response>>| {
				// println!(
				// 	"Running fallback predicate with req: {:?}, res: {:?}",
				// 	req.is_some(),
				// 	res.is_some()
				// );
				req.is_some() && res.is_none()
			},
		)
	}

	pub fn contains_handler_bundle() -> Self {
		Self::default().system(|query: Query<(), With<HandlerBundle>>| {
			query.iter().next().is_some()
		})
	}

	/// Runs if there is no [`Response`].
	pub fn no_response() -> Self {
		Self::default().system(|res: Option<Res<Response>>| res.is_none())
	}
	/// Runs if the router is in [`RenderMode::Ssr`].
	pub fn is_ssr() -> Self {
		Self::default().system(|render_mode: Res<RenderMode>| {
			*render_mode == RenderMode::Ssr
		})
	}

	pub fn system<Marker>(
		mut self,
		pred: impl 'static + Send + Sync + Clone + IntoSystem<(), bool, Marker>,
	) -> Self {
		self.0.push(Arc::new(move |mut world: World, _: Entity| {
			let pred = pred.clone();
			Box::pin(async move {
				match world.run_system_cached(pred) {
					Ok(out) => (world, out),
					Err(err) => {
						world.insert_resource(
							HttpError::from(err).into_response(),
						);
						(world, false)
					}
				}
			})
		}));
		self
	}
	pub fn entity_system<Marker>(
		mut self,
		pred: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ IntoSystem<In<Entity>, bool, Marker>,
	) -> Self {
		self.0
			.push(Arc::new(move |mut world: World, entity: Entity| {
				let pred = pred.clone();
				Box::pin(async move {
					match world.run_system_cached_with(pred, entity) {
						Ok(out) => (world, out),
						Err(err) => {
							world.insert_resource(
								HttpError::from(err).into_response(),
							);
							(world, false)
						}
					}
				})
			}));
		self
	}
	/// Returns false if any predicate returns false.
	pub async fn should_run(
		&self,
		mut world: World,
		entity: Entity,
	) -> (World, bool) {
		for pred in &self.0 {
			match pred(world, entity).await {
				(world, false) => return (world, false),
				(world2, true) => world = world2,
			}
		}
		(world, true)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[rustfmt::skip]
	#[sweet::test]
	async fn runs_async() {
		Router::new_bundle(|| {
			(
					HandlerConditions::fallback(),
					RouteHandler::endpoint(|| "fallback")
			)
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("fallback");
	}
	#[rustfmt::skip]
	#[sweet::test]
	async fn skips_async() {
		Router::new_bundle(|| {
			children![
				RouteHandler::endpoint(|| "endpoint"),
				(
					HandlerConditions::fallback(),
					RouteHandler::endpoint(|| "fallback")
				)
			]
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("endpoint");
	}
}
