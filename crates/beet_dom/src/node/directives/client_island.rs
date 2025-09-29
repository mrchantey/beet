use crate::prelude::*;
use bevy::ecs::component::Immutable;
use bevy::ecs::component::StorageType;
use bevy::ecs::lifecycle::ComponentHook;
use bevy::prelude::*;
use bevy::reflect::Reflectable;

/// A [`SceneFilter`] used to constrain the components serialized to the client scene,
/// by default only:
/// - [`ClientLoadDirective`]
/// - [`ClientOnlyDirective`]
/// - [`DomIdx`]
/// - [`ClientIslandRoot<T>`]
#[derive(Debug, Clone, Resource, Deref)]
pub struct ClientIslandRegistry(SceneFilter);

impl Default for ClientIslandRegistry {
	fn default() -> Self {
		Self(
			SceneFilter::default()
				.allow::<ClientLoadDirective>()
				.allow::<ClientOnlyDirective>()
				.allow::<DomIdx>()
				// required by apply_slots for debugging
				.allow::<NodeTag>(),
		)
	}
}


impl ClientIslandRegistry {
	pub fn add<T: Component>(&mut self) -> &mut Self {
		self.0 = self.0.clone().allow::<T>();
		self
	}
	pub fn filter(&self) -> SceneFilter { self.0.clone() }
}


/// Added to any template with a client directive, ie [`ClientLoadDirective`],
/// automatically registering the type for serialization in the client scene.
/// This also adds a [`TemplateRoot`]
#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ClientIslandRoot<T, U = T>
where
	T: 'static + Send + Sync + FromReflect + Reflectable + IntoBundle<U>,
	U: 'static + Send + Sync + TypePath,
{
	value: T,
	#[reflect(ignore)]
	phantom: std::marker::PhantomData<U>,
}

impl<T, U> ClientIslandRoot<T, U>
where
	T: 'static + Send + Sync + FromReflect + Reflectable + IntoBundle<U>,
	U: 'static + Send + Sync + TypePath,
{
	/// Create a new [`ClientIslandRoot<T>`] with the given value,
	/// alongside a [`TemplateRoot::spawn(T)`] using the same value.
	pub fn new(value: T) -> impl Bundle {
		Self {
			value,
			phantom: std::marker::PhantomData,
		}
	}
}


impl<T, U> Component for ClientIslandRoot<T, U>
where
	T: 'static + Send + Sync + FromReflect + Reflectable + IntoBundle<U>,
	U: 'static + Send + Sync + TypePath,
{
	const STORAGE_TYPE: StorageType = StorageType::Table;
	type Mutability = Immutable;

	// self register for the scene
	fn on_add() -> Option<ComponentHook> {
		Some(|mut world, cx| {
			let entity = cx.entity;
			world.commands().queue(move |world: &mut World| {
				// if the registry is locked we're probably in a scene load,
				// so we can skip the registration
				if let Ok(mut registry) =
					world.resource::<AppTypeRegistry>().internal.try_write()
				{
					// register the reflect type
					registry.register::<Self>();
				}
				// add to allow list
				world.resource_mut::<ClientIslandRegistry>().add::<Self>();
				// spawn the template root
				let this = world.entity(entity).get::<Self>().unwrap();
				// perform a 'reflect clone'
				let dynamic: Box<dyn PartialReflect> = this.value.to_dynamic();
				let value =
					<T as FromReflect>::from_reflect(&*dynamic).unwrap();
				world
					.entity_mut(entity)
					.insert(TemplateRoot::spawn(Spawn(value.into_bundle())));
			});
		})
	}
}
