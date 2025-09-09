#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::as_beet::*;
use sweet::prelude::*;


use bevy::ecs::bundle::BundleEffect;
use bevy::prelude::*;

#[test]
fn works() {
	#[derive(Debug, Component)]
	struct Bar;

	#[derive(Default, ImplBundle)]
	struct Foo<T: 'static + Send + Sync> {
		_phantom: std::marker::PhantomData<T>,
	}

	impl<T: 'static + Send + Sync> BundleEffect for Foo<T> {
		fn apply(self, entity: &mut EntityWorldMut) { entity.insert(Bar); }
	}

	let mut world = World::new();
	let entity = world.spawn(Foo::<()>::default());
	entity.get::<Bar>().xpect_some();
}
