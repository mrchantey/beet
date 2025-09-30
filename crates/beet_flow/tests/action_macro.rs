#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use beet_flow::prelude::*;
use sweet::prelude::*;

#[derive(Resource)]
struct Visited;

#[test]
fn works() {
	App::new()
		.world_mut()
		.spawn(Foo)
		.auto_trigger(RUN)
		.world_scope(|world| {
			world.resource::<Visited>();
		});
}

// #[action(foo)]
#[derive(Component)]
#[component(on_add = define_action)]
struct Foo;


fn foo(_: On<Run>, mut commands: Commands) {
	commands.insert_resource(Visited);
}


fn define_action(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.spawn(Observer::new(foo).with_entity(cx.entity));
	println!("here!");
}
