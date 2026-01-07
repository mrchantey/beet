#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use std::marker::PhantomData;

#[derive(Resource)]
struct Visited;

#[derive(EntityTargetEvent)]
struct Run<T: 'static + Send + Sync>(pub T);

/// Demonstrates declaring generic and non-generic actions in macro
#[test]
fn works() {
	App::new()
		.world_mut()
		.spawn(Foo::<bool>(Default::default()))
		.trigger_target(Run(true))
		.flush()
		.world_scope(|world| {
			world.get_resource::<Visited>().xpect_some();
		});
}

#[action(foo,bar::<T>)]
#[derive(Component)]
struct Foo<T: 'static + Send + Sync>(PhantomData<T>);

fn foo(_: On<Run<bool>>) {}

fn bar<T: 'static + Send + Sync>(_: On<Run<T>>, mut commands: Commands) {
	commands.insert_resource(Visited);
}
