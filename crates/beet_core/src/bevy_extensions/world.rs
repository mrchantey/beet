use crate::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::component::ComponentInfo;
use bevy::ecs::message::MessageCursor;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use extend::ext;
use std::marker::PhantomData;
/// system version
pub fn log_component_names(entity: In<Entity>, world: &mut World) {
	world.log_component_names(*entity);
}


/// common trait for 'App' and 'World'
pub trait IntoWorld {
	#[allow(unused)]
	fn into_world(&self) -> &World;
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
#[ext(name=WorldExt)]
pub impl World {
	fn with_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
		self.insert_resource(resource);
		self
	}

	fn await_insert<B: Bundle>(&mut self) -> impl Future<Output = &mut Self> {
		let (send, recv) = async_channel::bounded(1);
		self.add_observer(
			move |ready: On<Insert, B>, mut commands: Commands| {
				send.try_send(()).ok();
				commands.entity(ready.observer()).despawn();
			},
		);
		async move {
			AsyncRunner::poll_and_update(
				|| {
					self.update();
				},
				recv,
			)
			.await;
			self
		}
	}

	/// The world equivelent of [`App::update`]
	fn update(&mut self) { self.run_schedule(Main); }
	/// The world equivelent of [`App::should_exit`]
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

#[ext(name=IntoWorldMutExt)]
/// Matcher extensions for `bevy::World`
pub impl<W: IntoWorld> W {
	fn component_names(&self, entity: Entity) -> Vec<String> {
		let world = self.into_world();
		world
			.inspect_entity(entity)
			.map(|ent| {
				ent.map(|comp| self.pretty_name(comp)).collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}
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

	/// Try to get the short name of a component, otherwise return the full name.
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


	fn log_component_names(&self, entity: Entity) {
		let names = self.component_names_related::<Children>(entity);
		let str = names.iter_to_string_indented();
		println!("Component names for {entity}: \n{str}");
		// bevy::log::info!("Component names for {entity}: \n{str}");
	}

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



	/// Shorthand for creating a query and immediatly collecting it into a Vec.
	/// This is less efficient than caching the [`QueryState`] so should only be
	/// used for one-off queries, otherwise [`World::query`] should be preferred.
	fn query_once<D: QueryData>(&mut self) -> QueryOnce<D, ()> {
		QueryOnce::new(self)
	}

	/// Shorthand for creating a query and immediatly collecting it into a Vec.
	/// This is less efficient than caching the [`QueryState`] so should only be
	/// used for one-off queries, otherwise [`World::query_filtered`] should be preferred.
	fn query_filtered_once<D: QueryData, F: QueryFilter>(
		&mut self,
	) -> QueryOnce<D, F> {
		QueryOnce::new(self)
	}

	fn all_entities(&mut self) -> Vec<Entity> {
		let world = self.into_world_mut();
		world.query::<Entity>().iter(world).collect()
	}

	/// Shorthand for removing all components of a given type.
	fn remove<C: Component>(&mut self) -> Vec<C> {
		let world = self.into_world_mut();
		world
			.query_filtered::<Entity, With<C>>()
			.iter(world)
			.collect::<Vec<_>>()
			.into_iter()
			.filter_map(|entity| world.entity_mut(entity).take::<C>())
			.collect()
	}

	/// Shorthand for building a serialized scene from the current world.
	#[cfg(feature = "bevy_scene")]
	fn build_scene(&mut self) -> String {
		let all_entities = self.all_entities();
		let world = self.into_world();
		let dyn_scene = DynamicSceneBuilder::from_world(world)
			.deny_resource::<Time<Real>>()
			.extract_entities(all_entities.into_iter())
			.extract_resources()
			.build();


		self.build_scene_with(dyn_scene)
	}
	#[cfg(feature = "bevy_scene")]
	fn build_scene_with(&self, scene: DynamicScene) -> String {
		use bevy::scene::ron;
		use bevy::scene::serde::SceneSerializer;

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
	#[cfg(feature = "bevy_scene")]
	fn load_scene(&mut self, scene: impl AsRef<str>) -> Result {
		self.load_scene_with(scene, &mut Default::default())
	}
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
			let mut deserializer =
				bevy::scene::ron::de::Deserializer::from_str(scene)?;
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


	/// copied from world.trigger_ref_with_caller
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


#[cfg(test)]
#[cfg(feature = "bevy_scene")]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn serializes() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init();
		app.update();
		app.world_mut().build_scene().xpect_contains("Time");
	}
}
