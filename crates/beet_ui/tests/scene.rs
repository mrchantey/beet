beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;
// `use beet_ui::*` resolves the `crate::prelude::*` paths emitted by the
// `rsx_scene!` / `#[scene]` macros (mirrors tests/rsx.rs).
use beet_ui::*;

#[beet_core::test]
fn single_element() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx_scene! { <div/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
}

#[beet_core::test]
fn element_with_text_child() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx_scene! { <div>"hello"</div> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(1);
	world
		.entity(children[0])
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::new("hello"));
}

#[scene]
fn Card(#[prop(into)] title: String) -> impl Scene {
	rsx_scene! { <div class="card">{ template_value(Value::new(title)) }</div> }
}

#[beet_core::test]
fn scene_component_via_tag() {
	// `<Card title="Hi"/>` lowers to `Card(CardProps::default().with_title("Hi"))`
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx_scene! { <Card title="Hi"/> }).unwrap().id();

	// the root is the div returned by `Card`
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
	let attrs = world.entity(root).get::<Attributes>().unwrap();
	attrs.len().xpect_eq(1);
	(**world.entity(attrs[0]).get::<Attribute>().unwrap()).xpect_eq("class");

	// the prop flows through to the inner value child
	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(1);
	world
		.entity(children[0])
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::new("Hi"));
}

#[derive(Resource, Clone)]
struct AppTitle(String);

// app_info shape: no props, reads a resource synchronously at build, returns a
// sub-scene. The constructor is NOT async.
#[scene(system)]
fn AppInfo(config: Res<AppTitle>) -> impl Scene {
	let title = config.0.clone();
	rsx_scene! { <article>{ template_value(Value::new(title)) }</article> }
}

#[beet_core::test]
fn scene_system_reads_resource() {
	let mut world = scene_ext::test_world();
	world.insert_resource(AppTitle("beet".into()));

	// `<AppInfo/>` lowers to `AppInfo(AppInfoProps::default())`
	let root = world.spawn_scene(rsx_scene! { <AppInfo/> }).unwrap().id();

	// the resource value flowed into the built sub-scene synchronously
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("article");
	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(1);
	world
		.entity(children[0])
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::new("beet"));
}

#[beet_core::test]
fn event_attribute_attaches_observer() {
	#[derive(EntityEvent)]
	struct Ping(Entity);
	#[derive(Resource, Default)]
	struct Pinged(bool);

	let mut world = scene_ext::test_world();
	world.init_resource::<Pinged>();

	let root = world
		.spawn_scene(rsx_scene! {
			<button onclick={
				|_: On<Ping>, mut pinged: ResMut<Pinged>| pinged.0 = true
			}/>
		})
		.unwrap()
		.id();

	// the observer is attached but has not fired yet
	world.resource::<Pinged>().0.xpect_false();
	world.trigger(Ping(root));
	world.resource::<Pinged>().0.xpect_true();
}

#[beet_core::test]
fn nested_elements_with_attribute() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx_scene! { <div class="container"><span>"inner"</span></div> })
		.unwrap()
		.id();

	// root: <div class="container">
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
	let attrs = world.entity(root).get::<Attributes>().unwrap();
	attrs.len().xpect_eq(1);
	let attr = world.entity(attrs[0]);
	(**attr.get::<Attribute>().unwrap()).xpect_eq("class");
	attr.get::<Value>().unwrap().xpect_eq(Value::new("container"));

	// child: <span>"inner"</span>
	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(1);
	let span = world.entity(children[0]);
	span.get::<Element>().unwrap().tag().xpect_eq("span");
	let span_children = span.get::<Children>().unwrap();
	span_children.len().xpect_eq(1);
	world
		.entity(span_children[0])
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::new("inner"));
}
