//! Pins the order of bevy's component lifecycle hooks and observers, which is
//! hard to remember and could change quietly between bevy releases.

beet_core::test_main!();

use beet_core::prelude::*;
use beet_core::testing;

/// Each hook and observer pushes `(entity, label)` here as it runs.
#[derive(Default, Resource, Deref, DerefMut)]
struct Log(Vec<(Entity, &'static str)>);

/// A hook recording `label`, asserting the hooked component is present.
fn record_hook<C: Component>(
	label: &'static str,
) -> impl Fn(DeferredWorld, HookContext) {
	move |mut world, cx| {
		world.entity(cx.entity).contains::<C>().xpect_true();
		world.resource_mut::<Log>().push((cx.entity, label));
	}
}

#[derive(Component)]
#[component(on_add = record_hook::<Comp1>("Comp1 Hook Add"))]
#[component(on_remove = record_hook::<Comp1>("Comp1 Hook Remove"))]
struct Comp1;

#[derive(Component)]
#[component(on_add = record_hook::<Comp2>("Comp2 Hook Add"))]
#[component(on_remove = record_hook::<Comp2>("Comp2 Hook Remove"))]
struct Comp2;

/// A world observing add and remove of both components.
fn lifecycle_world() -> World {
	let mut world = World::new();
	world.init_resource::<Log>();
	world.add_observer(
		|ev: On<Add<Comp1>>, query: Query<&Comp1>, mut log: ResMut<Log>| {
			query.contains(ev.entity).xpect_true();
			log.push((ev.entity, "Comp1 Event Add"));
		},
	);
	world.add_observer(
		|ev: On<Remove<Comp1>>, query: Query<&Comp1>, mut log: ResMut<Log>| {
			query.contains(ev.entity).xpect_true();
			log.push((ev.entity, "Comp1 Event Remove"));
		},
	);
	world.add_observer(|ev: On<Add<Comp2>>, mut log: ResMut<Log>| {
		log.push((ev.entity, "Comp2 Event Add"));
	});
	world.add_observer(|ev: On<Remove<Comp2>>, mut log: ResMut<Log>| {
		log.push((ev.entity, "Comp2 Event Remove"));
	});
	world
}

/// Hooks run before observers on add and after them on remove, each phase
/// visiting the components in bundle order.
#[beet_core::test]
fn multi_component() {
	let mut world = lifecycle_world();
	let entity = world.spawn((Comp1, Comp2)).id();
	world.despawn(entity);
	world.resource::<Log>().0.clone().xpect_eq(vec![
		(entity, "Comp1 Hook Add"),
		(entity, "Comp2 Hook Add"),
		(entity, "Comp1 Event Add"),
		(entity, "Comp2 Event Add"),
		(entity, "Comp1 Event Remove"),
		(entity, "Comp2 Event Remove"),
		(entity, "Comp1 Hook Remove"),
		(entity, "Comp2 Hook Remove"),
	]);
}

/// A parent finishes each phase before its children start it.
#[beet_core::test]
fn child() {
	let mut world = lifecycle_world();
	let parent = world.spawn((Comp1, children![Comp1])).id();
	let child = world.entity(parent).get::<Children>().unwrap()[0];
	world.despawn(parent);
	world.resource::<Log>().0.clone().xpect_eq(vec![
		(parent, "Comp1 Hook Add"),
		(parent, "Comp1 Event Add"),
		(child, "Comp1 Hook Add"),
		(child, "Comp1 Event Add"),
		(parent, "Comp1 Event Remove"),
		(parent, "Comp1 Hook Remove"),
		(child, "Comp1 Event Remove"),
		(child, "Comp1 Hook Remove"),
	]);
}

#[derive(Component)]
#[component(on_add = on_add_foo)]
struct Foo;
/// Inserts [`Bazz`] through the hook's command queue.
fn on_add_foo(mut world: DeferredWorld, cx: HookContext) {
	world.resource_mut::<Log>().push((cx.entity, "Foo Hook"));
	world.commands().entity(cx.entity).insert(Bazz);
}

#[derive(Component)]
#[component(on_add = record_hook::<Bar>("Bar Hook"))]
struct Bar;

#[derive(Component)]
#[component(on_add = record_hook::<Bazz>("Bazz Hook"))]
struct Bazz;

/// A command a hook queues applies before the spawned children are built,
/// even when the children are listed first in the bundle.
#[beet_core::test]
fn hook_commands_before_children() {
	let mut world = World::new();
	world.init_resource::<Log>();
	world.add_observer(|ev: On<Insert<Foo>>, mut log: ResMut<Log>| {
		log.push((ev.entity, "Foo Observer"));
	});
	world.add_observer(|ev: On<Insert<Bar>>, mut log: ResMut<Log>| {
		log.push((ev.entity, "Bar Observer"));
	});
	world.add_observer(|ev: On<Insert<Bazz>>, mut log: ResMut<Log>| {
		log.push((ev.entity, "Bazz Observer"));
	});
	let parent = world.spawn((children![Bar], Foo)).id();
	let child = world.entity(parent).get::<Children>().unwrap()[0];
	world.resource::<Log>().0.clone().xpect_eq(vec![
		(parent, "Foo Hook"),
		(parent, "Foo Observer"),
		(parent, "Bazz Hook"),
		(parent, "Bazz Observer"),
		(child, "Bar Hook"),
		(child, "Bar Observer"),
	]);
}
