//! Not actually testing anything in beet_core but its very
//! hard to remember bevy's lifecycle rules.
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_core::prelude::*;


#[derive(Component)]
#[component(on_add=on_add1)]
#[component(on_remove=on_remove1)]
struct Comp1;

fn on_add1(world: DeferredWorld, cx: HookContext) {
	println!("{:?}: Comp1 Hook  - Add", cx.entity);
	world.entity(cx.entity).get::<Comp1>().unwrap();
}
fn on_remove1(world: DeferredWorld, cx: HookContext) {
	println!("{:?}: Comp1 Hook  - Remove", cx.entity);
	world.entity(cx.entity).get::<Comp1>().unwrap();
}
fn added1(ev: On<Add, Comp1>, query: Query<&Comp1>) {
	println!("{:?}: Comp1 Event - Add", ev.entity);
	query.get(ev.entity).unwrap();
}

fn removed1(ev: On<Remove, Comp1>, query: Query<&Comp1>) {
	println!("{:?}: Comp1 Event - Remove", ev.entity);
	query.get(ev.entity).unwrap();
}


#[derive(Component)]
#[component(on_add=on_add2)]
#[component(on_remove=on_remove2)]
struct Comp2;

fn on_add2(_world: DeferredWorld, cx: HookContext) {
	println!("{:?}: Comp2 Hook  - Add", cx.entity);
}
fn on_remove2(_world: DeferredWorld, cx: HookContext) {
	println!("{:?}: Comp2 Hook  - Remove", cx.entity);
}

fn added2(ev: On<Add, Comp2>) {
	println!("{:?}: Comp2 Event - Add", ev.entity);
}

fn removed2(ev: On<Remove, Comp2>) {
	println!("{:?}: Comp2 Event - Remove", ev.entity);
}


#[test]
#[ignore]
fn multi_component() {
	let mut world = World::new();
	world.add_observer(added1);
	world.add_observer(removed1);
	world.add_observer(added2);
	world.add_observer(removed2);
	world.spawn((Comp1, Comp2)).despawn();
	// 4v0: Comp1 Hook  - Add
	// 4v0: Comp2 Hook  - Add
	// 4v0: Comp1 Event - Add
	// 4v0: Comp2 Event - Add
	// 4v0: Comp1 Event - Remove
	// 4v0: Comp2 Event - Remove
	// 4v0: Comp1 Hook  - Remove
	// 4v0: Comp2 Hook  - Remove
}

#[test]
#[ignore]
fn child() {
	let mut world = World::new();
	world.add_observer(added1);
	world.add_observer(removed1);
	world.add_observer(added2);
	world.add_observer(removed2);
	world.spawn((Comp1, children![Comp1])).despawn();
	// 4v0: Comp1 Hook  - Add
	// 4v0: Comp1 Event - Add
	// 5v0: Comp1 Hook  - Add
	// 5v0: Comp1 Event - Add
	// 4v0: Comp1 Event - Remove
	// 4v0: Comp1 Hook  - Remove
	// 5v0: Comp1 Event - Remove
	// 5v0: Comp1 Hook  - Remove
}
