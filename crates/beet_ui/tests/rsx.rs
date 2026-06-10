//! Lowering tests for the single `rsx!` macro: asserts the lowered tree for
//! elements, components, templates, spreads, slots, values, and events. This is
//! the Task 2 gate (the merged macro lowering correctly to the substrate).
beet_core::test_main!();

use beet_core::prelude::*;
// resolve `crate::` aliasing emitted by the macros.
use beet_ui::*;
use beet_ui::prelude::*;

/// A spawn-capable template world.
fn world() -> World { test_world() }

/// Names of an entity's children, in order.
fn child_values(world: &World, entity: Entity) -> Vec<Value> {
	world
		.entity(entity)
		.get::<Children>()
		.into_iter()
		.flat_map(|children| children.iter())
		.filter_map(|child| world.entity(child).get::<Value>().cloned())
		.collect()
}

#[beet_core::test]
fn single_element() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <div/> }).id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
}

#[beet_core::test]
fn element_with_string_attribute() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <div class="card"/> }).id();
	world.with_state::<AttributeQuery, _>(|query| {
		let (_, value) = query.find(root, "class").unwrap();
		value.clone().xpect_eq(Value::new("card"));
	});
}

#[beet_core::test]
fn element_with_expr_attribute() {
	let mut world = world();
	let bang = 3;
	let root = world.spawn_template(rsx! { <div bang=bang/> }).id();
	world.with_state::<AttributeQuery, _>(|query| {
		query.find(root, "bang").unwrap().1.as_i64().unwrap().xpect_eq(3);
	});
}

#[beet_core::test]
fn element_with_text_child() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <div>"hello"</div> }).id();
	child_values(&world, root).xpect_eq(vec![Value::new("hello")]);
}

#[beet_core::test]
fn element_with_block_value_child() {
	let mut world = world();
	let title = "Title".to_string();
	let root = world.spawn_template(rsx! { <p>{title}</p> }).id();
	child_values(&world, root).xpect_eq(vec![Value::new("Title")]);
}

#[beet_core::test]
fn element_with_block_spread() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! { <div {Name::new("spread")}/> })
		.id();
	world.entity(root).get::<Name>().unwrap().as_str().xpect_eq("spread");
}

#[beet_core::test]
fn nested_elements() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! { <div><span>"inner"</span></div> })
		.id();
	let span = world.entity(root).get::<Children>().unwrap()[0];
	world.entity(span).get::<Element>().unwrap().tag().xpect_eq("span");
	child_values(&world, span).xpect_eq(vec![Value::new("inner")]);
}

#[beet_core::test]
fn multiple_root_elements() {
	let mut world = world();
	// multiple roots become a fragment spawning each as a child.
	let root = world.spawn_template(rsx! { <br/> <hr/> }).id();
	world.entity(root).get::<Children>().unwrap().len().xpect_eq(2);
}

#[beet_core::test]
fn event_attribute_attaches_observer() {
	#[derive(EntityEvent)]
	struct Ping(Entity);
	#[derive(Resource, Default)]
	struct Pinged(bool);

	let mut world = world();
	world.init_resource::<Pinged>();
	let root = world
		.spawn_template(rsx! {
			<button onclick={
				|_: On<Ping>, mut pinged: ResMut<Pinged>| pinged.0 = true
			}/>
		})
		.id();

	world.resource::<Pinged>().0.xpect_false();
	world.trigger(Ping(root));
	world.resource::<Pinged>().0.xpect_true();
}

// a reflect-patch component tag: `<MyComponent foo bar="x"/>` lowers to
// `MyComponent { foo: true.into(), bar: "x".into(), ..Default::default() }`.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default)]
#[allow(dead_code)]
struct MyComponent {
	foo: bool,
	bar: String,
}

#[beet_core::test]
fn component_tag_patches_over_default() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! { <MyComponent foo bar="hello"/> })
		.id();
	let comp = world.entity(root).get::<MyComponent>().unwrap();
	comp.foo.xpect_true();
	comp.bar.as_str().xpect_eq("hello");
}

#[beet_core::test]
fn component_spread_inserts_additional() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! { <MyComponent foo {Name::new("extra")}/> })
		.id();
	world.entity(root).get::<Name>().unwrap().as_str().xpect_eq("extra");
}

// a `#[template]` tag builds its subtree with input props.
#[template]
fn Card(#[prop(into)] title: String) -> impl Bundle {
	rsx! { <article class="card">{title}</article> }
}

#[beet_core::test]
fn template_tag_builds_with_props() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <Card title="Hi"/> }).id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("article");
	child_values(&world, root).xpect_eq(vec![Value::new("Hi")]);
}

#[beet_core::test]
fn doctype_node() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <!DOCTYPE html> }).id();
	world.entity(root).get::<Doctype>().unwrap();
}
