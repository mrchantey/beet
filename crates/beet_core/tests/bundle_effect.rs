#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use sweet::prelude::*;

#[test]
fn works() {
	#[derive(Debug, Component)]
	struct Bar;

	#[derive(Default, BundleEffect)]
	struct Foo<T: 'static + Send + Sync> {
		_phantom: std::marker::PhantomData<T>,
	}
	impl<T: 'static + Send + Sync> Foo<T> {
		fn effect(self, entity: &mut EntityWorldMut) { entity.insert(Bar); }
	}

	let mut world = World::new();
	let entity = world.spawn(Foo::<()>::default());
	entity.get::<Bar>().xpect_some();
}
