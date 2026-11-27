//! [`BodyComponents`]: one entity's components read off its body.
use super::dynamic_template::DocPathResolver;
use super::scene_reflect::EntityKeys;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
use bevy_reflect::Typed;
use core::any::TypeId;

/// The components an [`EntityBody`] holds, read against a type registry
/// without spawning its document: what a route scan hoists off a document's
/// root ([`DocParsers::root_components`]).
pub struct BodyComponents;

impl BodyComponents {
	/// The component `T` that `body` holds, patched over its default, read as
	/// its type's serde form or as markup authors it.
	///
	/// `Ok(None)` only when no key names `T`. One that does and will not read
	/// is an error, never a silent default.
	pub fn get<T: Default + Reflect + Typed>(
		body: &EntityBody,
		registry: &TypeRegistry,
	) -> Result<Option<T>> {
		let Some(value) = Self::named::<T>(body) else {
			return Ok(None);
		};
		let registration =
			registry.get(TypeId::of::<T>()).ok_or_else(|| {
				bevyhow!(
					"a document declares `{}`, which is absent from the registry",
					T::short_type_path()
				)
			})?;
		let patch = EntityKeys::new([]).read(registration, registry, value)?;
		let mut component = T::default();
		component.try_apply(patch.as_ref())?;
		Some(component).xok()
	}

	/// Insert every component `body` holds onto `entity`, patched over each
	/// type's default, as the document at `document` where it has one
	/// ([`DocPath`]).
	///
	/// A template or a resource is skipped, belonging to the document's build
	/// rather than its metadata. A name nothing registers is
	/// skipped, so a lean binary hoists the subset it links.
	pub fn insert(
		body: &EntityBody,
		entity: &mut EntityWorldMut,
		document: Option<&FwdPath>,
	) -> Result {
		if body.is_empty() {
			return OK;
		}
		let app_registry = entity
			.world()
			.get_resource::<AppTypeRegistry>()
			.cloned()
			.ok_or_else(|| {
				bevyhow!("a body's components require an `AppTypeRegistry`")
			})?;
		let registry = app_registry.read();
		let keys = EntityKeys::new([]);
		for (name, value) in body.iter() {
			let Some(registration) =
				DynamicTemplate::registration(&registry, name)
			else {
				debug!(
					"skipping `{name}`: no component of that name is registered in this binary"
				);
				continue;
			};
			let Some(reflect_component) = registration
				.data::<ReflectComponent>()
				.filter(|_| registration.data::<ReflectResource>().is_none())
			else {
				continue;
			};
			let mut component =
				keys.deserialize(registration, &registry, value)?;
			if let Some(document) = document {
				DocPathResolver { document }.visit(component.as_mut())?;
			}
			reflect_component.insert(entity, component.as_ref(), &registry);
		}
		OK
	}

	/// A bundle [inserting](Self::insert) every component `body` holds
	/// as the entity spawns, for a caller composing a route rather than
	/// holding the world.
	pub fn hoist(body: EntityBody) -> impl Bundle {
		OnSpawn::new(move |entity: &mut EntityWorldMut| {
			Self::insert(&body, entity, None)
		})
	}

	/// The value `body` keys by `T`, its full type path or its short one.
	fn named<T: TypePath>(body: &EntityBody) -> Option<&Value> {
		body.get(T::type_path())
			.or_else(|_| body.get(TypeName::of::<T>().short().as_str()))
			.ok()
	}
}
