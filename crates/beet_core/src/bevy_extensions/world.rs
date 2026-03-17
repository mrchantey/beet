//! Extension methods for Bevy's [`World`].


use crate::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::component::ComponentInfo;
use bevy::ecs::message::MessageCursor;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
#[cfg(feature = "multi_threaded")]
use bevy::ecs::schedule::ExecutorKind;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use core::marker::PhantomData;
use extend::ext;

/// System that logs component names for an entity.
pub fn log_component_names(entity: In<Entity>, world: &mut World) {
	world.log_component_names(*entity);
}

/// Extension trait adding utility methods to [`World`].
#[ext(name=WorldExt)]
pub impl World {
	/// Inserts a resource and returns self for chaining.
	fn with_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
		self.insert_resource(resource);
		self
	}

	/// Runs the world's main schedule in a loop until an [`AppExit`] is triggered.
	fn run_local(&mut self) -> AppExit {
		loop {
			self.update_local();
			if let Some(exit) = self.should_exit() {
				return exit;
			}
		}
	}

	/// Creates a new [`SystemState`] for the given system parameter type.
	fn state<T: SystemParam>(&mut self) -> SystemState<T> {
		SystemState::new(self)
	}

	/// The world equivalent of [`App::update`].
	///
	/// In multi_threaded mode, this temporarily sets all schedules to use
	/// single-threaded execution to avoid deadlocks when called from within
	/// async tasks on IoTaskPool.
	fn update_local(&mut self) {
		#[cfg(feature = "multi_threaded")]
		{
			// Temporarily force single-threaded execution for all schedules
			// to avoid deadlock when called from within a spawn_local task.
			self.force_single_threaded_schedules();
			self.run_schedule(Main);
			self.clear_trackers();
		}
		#[cfg(not(feature = "multi_threaded"))]
		{
			self.run_schedule(Main);
			self.clear_trackers();
		}
	}

	/// Forces all schedules in the world to use single-threaded execution.
	///
	/// This is necessary when running schedules from within async tasks
	/// to avoid deadlocks with bevy's parallel schedule executor.
	#[cfg(feature = "multi_threaded")]
	fn force_single_threaded_schedules(&mut self) {
		self.resource_scope(|_world, mut schedules: Mut<Schedules>| {
			for (_label, schedule) in schedules.iter_mut() {
				if schedule.get_executor_kind() == ExecutorKind::MultiThreaded {
					schedule.set_executor_kind(ExecutorKind::SingleThreaded);
				}
			}
		});
	}

	/// The world equivalent of [`App::should_exit`].
	fn should_exit(&self) -> Option<AppExit> {
		let mut reader = MessageCursor::default();

		let events = self.get_resource::<Messages<AppExit>>()?;
		let mut events = reader.read(events);

		if events.len() != 0 {
			return Some(
				events
					.find(|exit| exit.is_error())
					.cloned()
					.unwrap_or(AppExit::Success),
			);
		}

		None
	}
}


/// A collected query result that owns its data.
///
/// This is useful for one-off queries where caching the [`QueryState`] is not needed.
/// For repeated queries, prefer using [`World::query`] directly.
pub struct QueryOnce<D: QueryData, F: QueryFilter = ()> {
	items: Vec<D::Item<'static, 'static>>,
	_phantom: PhantomData<F>,
}

impl<D: QueryData, F: QueryFilter> core::ops::Deref for QueryOnce<D, F> {
	type Target = Vec<D::Item<'static, 'static>>;
	fn deref(&self) -> &<Self as core::ops::Deref>::Target { &self.items }
}

impl<D: QueryData, F: QueryFilter> core::ops::DerefMut for QueryOnce<D, F> {
	fn deref_mut(&mut self) -> &mut <Self as core::ops::Deref>::Target {
		&mut self.items
	}
}

impl<D: QueryData, F: QueryFilter> QueryOnce<D, F> {
	/// Creates a new [`QueryOnce`] by running a query and collecting the results.
	pub fn new(world: &mut World) -> Self {
		let mut query = world.query_filtered::<D, F>();
		let items = query.iter_mut(world).collect::<Vec<_>>();
		// SAFETY: We're extending the lifetime to 'static because we own the data
		// The query items are collected into owned data structures
		let items = unsafe { core::mem::transmute(items) };
		Self {
			items,
			_phantom: PhantomData,
		}
	}
}

impl<D: QueryData, F: QueryFilter> IntoIterator for QueryOnce<D, F> {
	type Item = D::Item<'static, 'static>;
	type IntoIter = alloc::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter { self.items.into_iter() }
}

impl<'a, D: QueryData, F: QueryFilter> IntoIterator for &'a QueryOnce<D, F> {
	type Item = &'a D::Item<'static, 'static>;
	type IntoIter = core::slice::Iter<'a, D::Item<'static, 'static>>;

	fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}

impl<'a, D: QueryData, F: QueryFilter> IntoIterator
	for &'a mut QueryOnce<D, F>
{
	type Item = &'a mut D::Item<'static, 'static>;
	type IntoIter = core::slice::IterMut<'a, D::Item<'static, 'static>>;

	fn into_iter(self) -> Self::IntoIter { self.items.iter_mut() }
}

/// Extension trait adding entity inspection and query utilities to [`World`].
#[ext(name=WorldMutExt)]
pub impl World {
	/// Returns the names of all components on the given entity.
	fn component_names(&self, entity: Entity) -> Vec<String> {
		self.inspect_entity(entity)
			.map(|ent| {
				ent.map(|comp| self.pretty_name(comp)).collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}

	/// Returns the component names for all direct relations of the given entity.
	fn direct_component_names_related<R: RelationshipTarget>(
		&self,
		entity: Entity,
	) -> Vec<Vec<String>> {
		self.entity(entity)
			.get::<R>()
			.map(|related| {
				related
					.iter()
					.filter_map(|entity| self.inspect_entity(entity).ok())
					.map(|component_iter| {
						component_iter
							.map(|component| self.pretty_name(component))
							.collect::<Vec<_>>()
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}

	/// Returns the short name of a component if available, otherwise the full name.
	fn pretty_name(&self, component: &ComponentInfo) -> String {
		let id = component.type_id();
		if let Some(id) = id {
			if let Some(type_registry) = self.get_resource::<AppTypeRegistry>()
			{
				if let Some(info) = type_registry.read().get_type_info(id) {
					return info.ty().short_path().to_string();
				}
			}
		}
		component.name().to_string()
	}

	/// Logs the component names for an entity and its descendants.
	fn log_component_names(&self, entity: Entity) {
		let names = self.component_names_related::<Children>(entity);
		let str = names.iter_to_string_indented();
		crate::cross_log!("Component names for {entity}: \n{str}");
	}

	/// Returns the component names for an entity and all its descendants as a tree.
	fn component_names_related<R: RelationshipTarget>(
		&self,
		entity: Entity,
	) -> Tree<Vec<String>> {
		fn recurse<'a, R: RelationshipTarget>(
			world: &'a World,
			entity: Entity,
			visited: &mut HashSet<Entity>,
		) -> Tree<Vec<String>> {
			if !visited.insert(entity) {
				return Tree::default(); // Prevent cycles
			}
			let value = world
				.inspect_entity(entity)
				.map(|component_iter| {
					component_iter
						.map(|component| world.pretty_name(component))
						.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			let children = world
				.entity(entity)
				.get::<R>()
				.map(|related| {
					related
						.iter()
						.map(|related_entity| {
							recurse::<R>(world, related_entity, visited)
						})
						.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			Tree::new_with_children(value, children)
		}
		recurse::<R>(self, entity, &mut default())
	}

	/// Creates a query and immediately collects results into a [`QueryOnce`].
	///
	/// Less efficient than caching [`QueryState`], so prefer [`World::query`]
	/// for repeated queries.
	fn query_once<D: QueryData>(&mut self) -> QueryOnce<D, ()> {
		QueryOnce::new(self)
	}

	/// Creates a filtered query and immediately collects results into a [`QueryOnce`].
	///
	/// Less efficient than caching [`QueryState`], so prefer [`World::query_filtered`]
	/// for repeated queries.
	fn query_filtered_once<D: QueryData, F: QueryFilter>(
		&mut self,
	) -> QueryOnce<D, F> {
		QueryOnce::new(self)
	}

	/// Returns all entities in the world.
	fn all_entities(&mut self) -> Vec<Entity> {
		self.query::<Entity>().iter(self).collect()
	}

	/// Removes all components of the given type from all entities
	/// and returns their values.
	fn take_all<C: Component>(&mut self) -> Vec<C> {
		self.query_filtered::<Entity, With<C>>()
			.iter(self)
			.collect::<Vec<_>>()
			.into_iter()
			.filter_map(|entity| self.entity_mut(entity).take::<C>())
			.collect()
	}

	/// Triggers an event with a reference, using the given caller location.
	// a public version of bevy's inbuilt one
	#[track_caller]
	fn trigger_ref_with_caller_pub<'a, E: Event>(
		&mut self,
		event: &mut E,
		trigger: &mut E::Trigger<'a>,
		caller: MaybeLocation,
	) {
		let event_key = self.register_event_key::<E>();
		// SAFETY: event_key was just registered and matches `event`
		unsafe {
			DeferredWorld::from(self as &mut World)
				.trigger_raw(event_key, event, trigger, caller);
		}
	}
}



/// Extension trait adding observer and trigger helpers to [`World`].
#[ext(name=CoreWorldExt)]
pub impl World {
	/// Returns self with an observer spawned.
	fn with_observer<E: Event, B: Bundle, M>(
		mut self,
		system: impl IntoObserverSystem<E, B, M>,
	) -> Self {
		self.spawn(Observer::new(system));
		self
	}

	/// Spawns an observer and returns self for chaining.
	fn observing<E: Event, B: Bundle, M>(
		&mut self,
		system: impl IntoObserverSystem<E, B, M>,
	) -> &mut Self {
		self.spawn(Observer::new(system));
		self
	}

	/// Flushes, triggers the event, then flushes again.
	// TODO deprecated, bevy 0.16 fixes this?
	fn flush_trigger<'a, E: Event<Trigger<'a>: Default>>(
		&mut self,
		event: E,
	) -> &mut Self {
		self.flush();
		self.trigger(event);
		self.flush();
		self
	}
}

/// Extension trait adding trigger helpers to [`EntityWorldMut`].
#[extend::ext]
pub impl<'w> EntityWorldMut<'w> {
	/// Flushes, triggers the event, then flushes again.
	fn flush_trigger<'a, E: Event<Trigger<'a>: Default>>(
		&mut self,
		event: E,
	) -> &mut Self {
		unsafe {
			let world = self.world_mut();
			world.flush();
			world.trigger(event);
			world.flush();
		}
		self
	}
}
