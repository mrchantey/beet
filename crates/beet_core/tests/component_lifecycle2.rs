//! Not actually testing anything in beet_core but its very
//! hard to remember bevy's lifecycle rules.

use beet_core::prelude::*;

use beet_core::testing;

#[beet_core::test]
// #[ignore]
fn works() {
	App::new()
		.add_observer(on_insert_foo)
		.add_observer(on_insert_bar)
		.add_observer(on_insert_bazz)
		.spawn(
			// children before parent, makes no difference
			(children![Bar], Foo),
		);
	// Foo: Hook
	// Foo: Observer
	// Bazz: Hook
	// Bazz: Observer
	// Bar: Hook
	// Bar: Observer
}

#[derive(Component)]
#[component(on_add=on_add_foo)]
struct Foo;
fn on_add_foo(mut world: DeferredWorld, cx: HookContext) {
	println!("Foo: Hook");
	world.commands().entity(cx.entity).insert(Bazz);
}

fn on_insert_foo(_: On<Insert, Foo>) { println!("Foo: Observer") }

#[derive(Component)]
#[component(on_add=on_add_bar)]
struct Bar;
fn on_add_bar(_world: DeferredWorld, _cx: HookContext) { println!("Bar: Hook") }
fn on_insert_bar(_: On<Insert, Bar>) { println!("Bar: Observer") }

#[derive(Component)]
#[component(on_add=on_add_bazz)]
struct Bazz;
fn on_add_bazz(_world: DeferredWorld, _cx: HookContext) {
	println!("Bazz: Hook")
}
fn on_insert_bazz(_: On<Insert, Bazz>) { println!("Bazz: Observer") }

beet_core::test_main!();
