/// Implement an empty [`Bundle`] for something that should only be a [`BundleEffect`].
///
/// ## Example
///
/// ```
/// # use sweet_bevy::prelude::*;
/// # use bevy::prelude::*;
/// # use bevy::ecs::bundle::BundleEffect;
///
/// struct Foo;
///	effect_bundle!(Foo);
///
///	impl BundleEffect for Foo {
///		fn apply(self, _: &mut EntityWorldMut) { }
///	}
///
/// ```
#[macro_export]
macro_rules! effect_bundle {
	($type:ty) => {
		unsafe impl bevy::ecs::bundle::Bundle for $type {
			fn component_ids(
				_: &mut bevy::ecs::component::ComponentsRegistrator,
				_: &mut impl FnMut(bevy::ecs::component::ComponentId),
			) {
			}

			fn get_component_ids(
				_: &bevy::ecs::component::Components,
				_: &mut impl FnMut(Option<bevy::ecs::component::ComponentId>),
			) {
			}

			fn register_required_components(
				_: &mut bevy::ecs::component::ComponentsRegistrator,
				_: &mut bevy::ecs::component::RequiredComponents,
			) {
			}
		}

		impl bevy::ecs::bundle::DynamicBundle for $type {
			type Effect = Self;
			fn get_components(
				self,
				_: &mut impl FnMut(
					bevy::ecs::component::StorageType,
					bevy::ecs::ptr::OwningPtr<'_>,
				),
			) -> Self::Effect {
				self
			}
		}
	};
}

#[cfg(test)]
mod test {
	use bevy::ecs::bundle::BundleEffect;
	use bevy::prelude::*;
	use sweet_test::prelude::*;

	#[test]
	fn works() {
		#[derive(Debug, Component)]
		struct Bar;

		struct Foo;
		effect_bundle!(Foo);

		impl BundleEffect for Foo {
			fn apply(self, entity: &mut EntityWorldMut) { entity.insert(Bar); }
		}

		let mut world = World::new();
		let entity = world.spawn(Foo);
		entity.get::<Bar>().xpect().to_be_some();
	}
}
