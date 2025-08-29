use crate::prelude::*;
// use beet_rsx::prelude::*;
use beet_utils::utils::PipelineTarget;
use bevy::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;


/// An async function that accepts and returns an owned [`World`], to be run
/// by the [`AsyncActionSet`].
///
#[derive(Clone, Component)]
pub struct AsyncAction(Arc<AsyncActionFunc>);

impl AsyncAction {
	/// Create a new async action.
	pub fn new<Func, Fut>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Clone + FnOnce(World, Entity) -> Fut,
		Fut: 'static + Send + Future<Output = World>,
	{
		Self(Arc::new(move |world, entity| {
			Box::pin(func.clone()(world, entity))
		}))
	}
	pub fn run(
		&self,
		world: World,
		entity: Entity,
	) -> Pin<Box<dyn Send + Future<Output = World>>> {
		(self.0)(world, entity)
	}
}

type AsyncActionFunc = dyn 'static
	+ Send
	+ Sync
	+ Fn(World, Entity) -> Pin<Box<dyn Send + Future<Output = World>>>;


/// A set of collected [`AsyncAction`] to be run by the [`AsyncRunner`].
/// These may be ordered or unordered depending on the construction method.
pub struct AsyncActionSet(pub Vec<(Entity, AsyncAction)>);

impl AsyncActionSet {
	pub async fn collect_and_run(mut world: World) -> World {
		let set = world
			.run_system_cached(Self::collect)
			.unwrap(/*infallible system params */);
		set.run(world).await
	}


	/// Collect all [`AsyncAction`] in a breadth-first search order.
	/// Any entity without a [`ChildOf`] component is considered a root.
	pub fn collect(
		roots: Query<Entity, Without<ChildOf>>,
		actions: Query<&AsyncAction>,
		children: Query<&Children>,
	) -> Self {
		roots
			.iter()
			.flat_map(|root| {
				children
					.iter_descendants_inclusive(root)
					.filter_map(|child| {
						actions
							.get(child)
							.ok()
							.map(|action| (child, action.clone()))
					})
			})
			.collect::<Vec<_>>()
			.xmap(Self)
	}

	pub fn collect_depth_first(
		roots: Query<Entity, Without<ChildOf>>,
		actions: Query<&AsyncAction>,
		children: Query<&Children>,
	) -> Self {
		roots
			.iter()
			.flat_map(|root| {
				children
					.iter_descendants_inclusive_depth_first(root)
					.filter_map(|child| {
						actions
							.get(child)
							.ok()
							.map(|action| (child, action.clone()))
					})
			})
			.collect::<Vec<_>>()
			.xmap(Self)
	}
	pub fn collect_unordered(actions: Query<(Entity, &AsyncAction)>) -> Self {
		actions
			.iter()
			.map(|(e, a)| (e, a.clone()))
			.collect::<Vec<_>>()
			.xmap(Self)
	}

	/// Run all actions in the set, returning the final world state.
	pub async fn run(self, mut world: World) -> World {
		for (entity, action) in self.0 {
			world = action.run(world, entity).await;
		}
		world
	}
}


pub trait IntoAsyncAction<M> {
	fn into_async_action(self) -> AsyncAction;
}

pub struct FutIntoAsyncAction;

impl<T, Fut> IntoAsyncAction<(T, Fut, FutIntoAsyncAction)> for T
where
	T: 'static + Send + Sync + Clone + FnOnce(World, Entity) -> Fut,
	Fut: 'static + Send + Sync + Future<Output = World>,
{
	fn into_async_action(self) -> AsyncAction { AsyncAction::new(self) }
}

pub struct SystemIntoAsyncAction;

impl<T, Marker> IntoAsyncAction<(T, Marker, SystemIntoAsyncAction)> for T
where
	T: 'static + Send + Sync + Clone + IntoSystem<(), (), Marker>,
{
	fn into_async_action(self) -> AsyncAction {
		AsyncAction::new(async move |mut world, _| {
			// run system errors are discarded, same behavior as schedules
			world.run_system_cached(self.clone()).ok();
			world
		})
	}
}
pub struct EntityInSystemIntoAsyncAction;

impl<T, Marker> IntoAsyncAction<(T, Marker, EntityInSystemIntoAsyncAction)>
	for T
where
	T: 'static + Send + Sync + Clone + IntoSystem<In<Entity>, (), Marker>,
{
	fn into_async_action(self) -> AsyncAction {
		AsyncAction::new(async move |mut world, entity| {
			// run system errors are discarded, same behavior as schedules
			world.run_system_cached_with(self.clone(), entity).ok();
			world
		})
	}
}

#[cfg(test)]
// currently must be Send
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use std::time::Duration;

	use crate::prelude::*;
	use beet_utils::time_ext::sleep;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		let entity = world
			.spawn(AsyncAction::new(async |mut world, entity| {
				// crosses async boundary
				sleep(Duration::from_millis(1)).await;
				world.entity_mut(entity).insert(Name::new("Hello"));
				world
			}))
			.id();
		world = AsyncActionSet::collect_and_run(world).await;
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.xpect()
			.to_be(&Name::new("Hello"));
	}
}
