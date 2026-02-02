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
use bevy::prelude::*;
use extend::ext;
use std::marker::PhantomData;

/// System that logs component names for an entity.
pub fn log_component_names(entity: In<Entity>, world: &mut World) {
	world.log_component_names(*entity);
}


/// Trait for types that can provide a reference to a [`World`].
pub trait IntoWorld {
	/// Returns a reference to the world.
	#[allow(unused)]
	fn into_world(&self) -> &World;
	/// Returns a mutable reference to the world.
	fn into_world_mut(&mut self) -> &mut World;
}
impl IntoWorld for World {
	fn into_world(&self) -> &World { self }
	fn into_world_mut(&mut self) -> &mut World { self }
}
impl IntoWorld for App {
	fn into_world(&self) -> &World { self.world() }
	fn into_world_mut(&mut self) -> &mut World { self.world_mut() }
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

impl<D: QueryData, F: QueryFilter> std::ops::Deref for QueryOnce<D, F> {
	type Target = Vec<D::Item<'static, 'static>>;
	fn deref(&self) -> &Self::Target { &self.items }
}

impl<D: QueryData, F: QueryFilter> std::ops::DerefMut for QueryOnce<D, F> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.items }
}

impl<D: QueryData, F: QueryFilter> QueryOnce<D, F> {
	/// Creates a new [`QueryOnce`] by running a query and collecting the results.
	pub fn new<T: IntoWorld>(world: &mut T) -> Self {
		let world = world.into_world_mut();
		let mut query = world.query_filtered::<D, F>();
		let items = query.iter_mut(world).collect::<Vec<_>>();
		// SAFETY: We're extending the lifetime to 'static because we own the data
		// The query items are collected into owned data structures
		let items = unsafe { std::mem::transmute(items) };
		Self {
			items,
			_phantom: PhantomData,
		}
	}
}

impl<D: QueryData, F: QueryFilter> IntoIterator for QueryOnce<D, F> {
	type Item = D::Item<'static, 'static>;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter { self.items.into_iter() }
}

impl<'a, D: QueryData, F: QueryFilter> IntoIterator for &'a QueryOnce<D, F> {
	type Item = &'a D::Item<'static, 'static>;
	type IntoIter = std::slice::Iter<'a, D::Item<'static, 'static>>;

	fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}

impl<'a, D: QueryData, F: QueryFilter> IntoIterator
	for &'a mut QueryOnce<D, F>
{
	type Item = &'a mut D::Item<'static, 'static>;
	type IntoIter = std::slice::IterMut<'a, D::Item<'static, 'static>>;

	fn into_iter(self) -> Self::IntoIter { self.items.iter_mut() }
}

/// Extension trait adding entity inspection and query utilities to [`World`] and [`App`].
#[ext(name=IntoWorldMutExt)]
pub impl<W: IntoWorld> W {
	/// Returns the names of all components on the given entity.
	fn component_names(&self, entity: Entity) -> Vec<String> {
		let world = self.into_world();
		world
			.inspect_entity(entity)
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
		let world = self.into_world();
		world
			.entity(entity)
			.get::<R>()
			.map(|related| {
				related
					.iter()
					.filter_map(|entity| world.inspect_entity(entity).ok())
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
			if let Some(type_registry) =
				self.into_world().get_resource::<AppTypeRegistry>()
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
		println!("Component names for {entity}: \n{str}");
		// bevy::log::info!("Component names for {entity}: \n{str}");
	}

	/// Returns the component names for an entity and all its descendants as a tree.
	fn component_names_related<R: RelationshipTarget>(
		&self,
		entity: Entity,
	) -> Tree<Vec<String>> {
		fn recurse<'a, R: RelationshipTarget>(
			world: &'a World,
			entity: Entity,
			visited: &mut std::collections::HashSet<Entity>,
		) -> Tree<Vec<String>> {
			if !visited.insert(entity) {
				return Tree::default(); // Prevent cycles
			}
			// Inspect the entity itself
			let value = world
				.inspect_entity(entity)
				.map(|component_iter| {
					component_iter
						.map(|component| world.pretty_name(component))
						.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			// Recurse into related entities
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
		recurse::<R>(self.into_world(), entity, &mut default())
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
		let world = self.into_world_mut();
		world.query::<Entity>().iter(world).collect()
	}

	/// Removes all components of the given type from all entities
	/// and returns their values.
	fn take_all<C: Component>(&mut self) -> Vec<C> {
		let world = self.into_world_mut();
		world
			.query_filtered::<Entity, With<C>>()
			.iter(world)
			.collect::<Vec<_>>()
			.into_iter()
			.filter_map(|entity| world.entity_mut(entity).take::<C>())
			.collect()
	}

	/// Builds a serialized scene from the current world.
	#[cfg(feature = "bevy_scene")]
	fn build_scene(&mut self) -> String {
		self.build_scene_with_builder(|builder| {
			builder.deny_resource::<Time<Real>>()
		})
	}

	/// Builds a serialized scene with a custom builder configuration.
	#[cfg(feature = "bevy_scene")]
	fn build_scene_with_builder(
		&mut self,
		func: impl FnOnce(DynamicSceneBuilder) -> DynamicSceneBuilder,
	) -> String {
		let all_entities = self.all_entities();
		let world = self.into_world();
		let dyn_scene = func(DynamicSceneBuilder::from_world(world))
			.extract_entities(all_entities.into_iter())
			.extract_resources()
			.build();


		self.build_scene_with(dyn_scene)
	}

	/// Serializes a [`DynamicScene`] to a RON string.
	#[cfg(feature = "bevy_scene")]
	fn build_scene_with(&self, scene: DynamicScene) -> String {
		use bevy::scene::serde::SceneSerializer;
		use ron;

		let world = self.into_world();
		let type_registry = world.resource::<AppTypeRegistry>();
		let type_registry = type_registry.read();
		let scene_serializer = SceneSerializer::new(&scene, &type_registry);
		let pretty_config = ron::ser::PrettyConfig::default()
			.indentor("  ".to_string())
			.new_line("\n".to_string());
		let scene =
			ron::ser::to_string_pretty(&scene_serializer, pretty_config)
				.expect("failed to serialize scene");
		scene
	}

	/// Loads a scene from a serialized RON string.
	#[cfg(feature = "bevy_scene")]
	fn load_scene(&mut self, scene: impl AsRef<str>) -> Result {
		self.load_scene_with(scene, &mut Default::default())
	}

	/// Loads a scene from a serialized RON string with a custom entity map.
	#[cfg(feature = "bevy_scene")]
	fn load_scene_with(
		&mut self,
		scene: impl AsRef<str>,
		entity_map: &mut bevy::ecs::entity::EntityHashMap<Entity>,
	) -> Result {
		let scene = scene.as_ref();
		let world = self.into_world_mut();
		let scene = {
			use serde::de::DeserializeSeed;
			let type_registry = world.resource::<AppTypeRegistry>();
			let mut deserializer = ron::de::Deserializer::from_str(scene)?;
			let scene_deserializer = bevy::scene::serde::SceneDeserializer {
				type_registry: &type_registry.read(),
			};

			scene_deserializer
				.deserialize(&mut deserializer)
				.map_err(|e| deserializer.span_error(e))
		}?;
		scene.write_to_world(world, entity_map)?;

		Ok(())
	}


	/// Triggers an event with a reference, using the given caller location.
	#[track_caller]
	fn trigger_ref_with_caller_pub<'a, E: Event>(
		&mut self,
		event: &mut E,
		trigger: &mut E::Trigger<'a>,
		caller: MaybeLocation,
	) {
		let world = self.into_world_mut();
		let event_key = world.register_event_key::<E>();
		// SAFETY: event_key was just registered and matches `event`
		unsafe {
			DeferredWorld::from(world)
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
		// let entity = self.id();
		unsafe {
			let world = self.world_mut();
			world.flush();
			world.trigger(event);
			world.flush();
		}
		self
	}
}


#[cfg(test)]
#[cfg(feature = "bevy_scene")]
mod test {
	use crate::prelude::*;

	#[test]
	fn serializes() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init();
		app.update();
		app.world_mut().build_scene().xpect_contains("Time");
	}
}
