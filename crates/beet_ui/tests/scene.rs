beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;
// `use beet_ui::*` resolves the `crate::prelude::*` paths emitted by the
// `rsx!` / `#[scene]` macros (mirrors tests/rsx.rs).
use beet_ui::*;

#[beet_core::test]
fn single_element() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <div/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
}

#[beet_core::test]
fn element_with_text_child() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <div>"hello"</div> }).unwrap().id();
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
	rsx! { <div class="card">{title}</div> }
}

#[beet_core::test]
fn scene_component_via_tag() {
	// `<Card title="Hi"/>` lowers to `Card(CardProps::default().with_title("Hi"))`
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Card title="Hi"/> }).unwrap().id();

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
	rsx! { <article>{title}</article> }
}

#[beet_core::test]
fn scene_system_reads_resource() {
	let mut world = scene_ext::test_world();
	world.insert_resource(AppTitle("beet".into()));

	// `<AppInfo/>` lowers to `AppInfo(AppInfoProps::default())`
	let root = world.spawn_scene(rsx! { <AppInfo/> }).unwrap().id();

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
		.spawn_scene(rsx! {
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

/// `beet_design::Button` reduced to its scene shape: an enum-keyed semantic
/// class attached via the [`Classes`] component (not a `class="…"` string),
/// plus an `#[prop(into)]` String for the label. Demonstrates the
/// `#[template]` → `#[scene]` migration pattern; the actual visual rules ship
/// with `beet_design.md` Phase 3.
#[derive(Default, Clone)]
#[allow(dead_code)]
enum ButtonVariant {
	#[default]
	Filled,
	Outlined,
	Error,
}

impl ButtonVariant {
	fn class(&self) -> &'static str {
		match self {
			ButtonVariant::Filled => "btn-filled",
			ButtonVariant::Outlined => "btn-outlined",
			ButtonVariant::Error => "btn-error",
		}
	}
}

#[scene]
fn Button(
	#[prop(into)] label: String,
	variant: ButtonVariant,
) -> impl Scene {
	rsx! {
		<button {Classes::new(["btn", variant.class()])}>
			{label}
		</button>
	}
}

#[beet_core::test]
fn button_widget_renders_with_props() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <Button label="Save" variant=ButtonVariant::Error/> })
		.unwrap()
		.id();

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("button");

	// the marker Component is on the root entity — unlocks queries +
	// `:Button` inheritance from BSN
	world.entity(root).get::<Button>().unwrap();

	// classes attached via the `Classes` component, not a `class="…"` string
	let classes = world.entity(root).get::<Classes>().unwrap();
	classes.contains_selector("btn").xpect_true();
	classes.contains_selector("btn-error").xpect_true();
	classes.contains_selector("btn-filled").xpect_false();

	// label prop became a text child
	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(1);
	world
		.entity(children[0])
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::new("Save"));
}

#[beet_core::test]
fn bsn_inheritance_matches_rsx_tag_form() {
	use bevy::scene::bsn;

	// rsx form
	let mut world = scene_ext::test_world();
	let rsx_root = world
		.spawn_scene(rsx! { <Button label="Save" variant=ButtonVariant::Error/> })
		.unwrap()
		.id();

	// hand-written BSN form — `:Button` inherits the SceneComponent and `@`
	// fields set props. Should produce the same tree.
	let bsn_root = world
		.spawn_scene(bsn! {
			:Button { @label: {"Save".to_string()}, @variant: {ButtonVariant::Error} }
		})
		.unwrap()
		.id();

	let rsx_button = world.entity(rsx_root).get::<Element>().unwrap().tag().to_string();
	let bsn_button = world.entity(bsn_root).get::<Element>().unwrap().tag().to_string();
	rsx_button.xpect_eq(bsn_button);

	// both carry the marker Component
	world.entity(rsx_root).get::<Button>().unwrap();
	world.entity(bsn_root).get::<Button>().unwrap();
}

/// `beet_design::BucketList` reduced to its essence: a synchronous `#[scene]`
/// constructor whose **behavior** (observer firing → mutating a signal-driven
/// resource) attaches via an event attribute on the scene-built tree. No part
/// of the constructor is async.
#[scene]
fn Counter(#[prop(into)] label: String) -> impl Scene {
	rsx! {
		<button
			{Classes::new(["btn", "btn-counter"])}
			onclick={
				|_: On<Bump>, mut count: ResMut<Count>| count.0 += 1
			}
		>
			{label}
		</button>
	}
}

#[derive(EntityEvent)]
struct Bump(Entity);

#[derive(Resource, Default)]
struct Count(u32);

#[beet_core::test]
fn counter_widget_behavior_attaches_to_scene_built_tree() {
	let mut world = scene_ext::test_world();
	world.init_resource::<Count>();

	let root = world
		.spawn_scene(rsx! { <Counter label="Bump"/> })
		.unwrap()
		.id();

	world.resource::<Count>().0.xpect_eq(0);
	world.trigger(Bump(root));
	world.trigger(Bump(root));
	world.resource::<Count>().0.xpect_eq(2);
}

#[beet_core::test]
fn nested_elements_with_attribute() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <div class="container"><span>"inner"</span></div> })
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
