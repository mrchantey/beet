beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;
use beet_ui::prelude::classes;
// explicit so `spawn_scene` resolves to beet_ui's slot-wiring trait, not the
// `bevy::scene` one also glob-imported via `beet_core::prelude`.
use beet_ui::prelude::WorldSceneExt;
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

/// A throwaway fixture for the generic scene-machinery checks below (marker
/// component, BSN inheritance, required/optional props). The *production*
/// button widget lives in `beet_ui::widgets::button`; this stays local so the
/// BSN form can set props by field (`@variant`) without exposing the
/// production `*Props` fields.
#[derive(Default, Clone)]
#[allow(dead_code)]
enum DemoVariant {
	#[default]
	Filled,
	Outlined,
	Error,
}

impl DemoVariant {
	fn class(&self) -> ClassName {
		match self {
			DemoVariant::Filled => classes::BTN_FILLED,
			DemoVariant::Outlined => classes::BTN_OUTLINED,
			DemoVariant::Error => classes::BTN_ERROR,
		}
	}
}

#[scene]
fn DemoButton(
	#[prop(into)] label: String,
	variant: DemoVariant,
) -> impl Scene {
	rsx! {
		<button {Classes::new([ClassName::new_static("btn"), variant.class()])}>
			{label}
		</button>
	}
}

#[beet_core::test]
fn button_widget_renders_with_props() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <DemoButton label="Save" variant=DemoVariant::Error/> })
		.unwrap()
		.id();

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("button");

	// the marker Component is on the root entity — unlocks queries +
	// `:DemoButton` inheritance from BSN
	world.entity(root).get::<DemoButton>().unwrap();

	// classes attached via the `Classes` component, not a `class="…"` string.
	// Assert against the shared `classes` constants to stay in lockstep with the
	// style rules that target them.
	let button_classes = world.entity(root).get::<Classes>().unwrap();
	button_classes.contains_selector("btn").xpect_true();
	button_classes.contains_name(&classes::BTN_ERROR).xpect_true();
	button_classes.contains_name(&classes::BTN_FILLED).xpect_false();

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
		.spawn_scene(rsx! { <DemoButton label="Save" variant=DemoVariant::Error/> })
		.unwrap()
		.id();

	// hand-written BSN form — `:DemoButton` inherits the SceneComponent and `@`
	// fields set props. Should produce the same tree.
	let bsn_root = world
		.spawn_scene(bsn! {
			:DemoButton { @label: {"Save".to_string()}, @variant: {DemoVariant::Error} }
		})
		.unwrap()
		.id();

	let rsx_button = world.entity(rsx_root).get::<Element>().unwrap().tag().to_string();
	let bsn_button = world.entity(bsn_root).get::<Element>().unwrap().tag().to_string();
	rsx_button.xpect_eq(bsn_button);

	// both carry the marker Component
	world.entity(rsx_root).get::<DemoButton>().unwrap();
	world.entity(bsn_root).get::<DemoButton>().unwrap();
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

/// A widget with a `#[prop(required)]` prop, exercising the runtime-checked
/// required-prop path (see `agent/plans/required_props.md`). Required-ness is
/// validated at build time: an unset required prop surfaces [`MissingProps`]
/// through the build channel rather than panicking.
#[scene]
fn Badge(#[prop(required)] variant: DemoVariant) -> impl Scene {
	rsx! { <span {Classes::new([ClassName::new_static("badge"), variant.class()])}/> }
}

/// Spawn `scene` into `world`, returning the root id or the build error as a
/// string — shared so the supplied and missing-prop cases stay DRY.
fn try_spawn(world: &mut World, scene: impl Scene) -> Result<Entity, String> {
	world.spawn_scene(scene).map(|entity| entity.id()).map_err(|err| err.to_string())
}

#[beet_core::test]
fn required_prop_supplied_resolves() {
	let mut world = scene_ext::test_world();
	let root = try_spawn(&mut world, rsx! { <Badge variant=DemoVariant::Error/> })
		.unwrap();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("span");
	world.entity(root).get::<Badge>().unwrap();
}

#[beet_core::test]
fn missing_required_prop_surfaces_error() {
	let mut world = scene_ext::test_world();
	// `<Badge/>` lowers to `BadgeProps::default()`, leaving `variant` unset
	let err = try_spawn(&mut world, rsx! { <Badge/> }).unwrap_err();
	err.xpect_contains("missing required props").xpect_contains("variant");
}

/// Exercises optional-prop ergonomics: `#[prop(default = expr)]` seeds a
/// non-`Default` value, and a bare `Option<T>` field defaults to `None` while
/// its setter accepts the inner `T`.
#[scene]
fn Tag(
	#[prop(default = "tag")] kind: String,
	label: Option<String>,
) -> impl Scene {
	let label = label.unwrap_or_default();
	rsx! { <span class={kind}>{label}</span> }
}

#[beet_core::test]
fn default_and_option_props() {
	let mut world = scene_ext::test_world();

	// omitted props fall back: `kind` -> "tag", `label` -> None (empty child)
	let root = world.spawn_scene(rsx! { <Tag/> }).unwrap().id();
	world.with_state::<ElementQuery, _>(|query| {
		query.get(root).unwrap().attribute_string("class").xpect_eq("tag");
	});

	// supplied props flow through, `label` setter takes `&str` (unwrap_option)
	let root = world.spawn_scene(rsx! { <Tag kind="x" label="hi"/> }).unwrap().id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.attribute_string("class").xpect_eq("x");
		query
			.iter_descendant_values(root)
			.any(|value| value.as_str().ok() == Some("hi"))
			.xpect_true();
	});
}

/// A custom props type supplied via `#[prop(all)]` — the macro emits no props
/// struct, treating `LabelProps` as the props type directly.
#[derive(Default, SetWith)]
struct LabelProps {
	#[set_with(into)]
	text: String,
}

#[scene]
fn Label(#[prop(all)] props: LabelProps) -> impl Scene {
	rsx! { <span>{props.text}</span> }
}

#[beet_core::test]
fn prop_all_uses_custom_props_type() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Label text="hi"/> }).unwrap().id();
	world.entity(root).get::<Label>().unwrap();
	world.with_state::<ElementQuery, _>(|query| {
		query
			.iter_descendant_values(root)
			.any(|value| value.as_str().ok() == Some("hi"))
			.xpect_true();
	});
}

#[beet_core::test]
fn element_with_more_than_twelve_children() {
	// 20 children exceeds `bevy_scene`'s 12-tuple cap; the rsx! lowering chunks
	// them into nested tuples so all spawn as direct children.
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! {
			<ul>
				<li/><li/><li/><li/><li/><li/><li/><li/><li/><li/>
				<li/><li/><li/><li/><li/><li/><li/><li/><li/><li/>
			</ul>
		})
		.unwrap()
		.id();
	world.entity(root).get::<Children>().unwrap().len().xpect_eq(20);
}

/// A widget with a named `header` slot and a default slot. The `#[scene]` macro
/// hoists each `<slot>` into a `SceneProp` prop (`header`, `children`), and the
/// caller fills them via `slot="header"` / unmarked content.
#[scene]
fn Panel() -> impl Scene {
	rsx! {
		<section>
			<header><slot name="header"/></header>
			<div><slot/></div>
		</section>
	}
}

/// A widget whose slots carry fallback content, rendered when the prop is unset.
#[scene]
fn FallbackPanel() -> impl Scene {
	rsx! {
		<section>
			<header><slot name="header">"Default Title"</slot></header>
			<div><slot>"Default Body"</slot></div>
		</section>
	}
}

/// Render `root` to an HTML string.
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}

#[beet_core::test]
fn children_as_props_default_and_named() {
	// `slot="header"` routes to the `header` prop; unmarked content fills the
	// default `children` prop. Each is placed where the widget puts it.
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! {
			<Panel>
				<h1 slot="header">"Title"</h1>
				<p>"Body"</p>
			</Panel>
		})
		.unwrap()
		.id();
	let html = render_html(&mut world, root);
	// the title lands inside <header>, the body inside the <div>; the transparent
	// prop wrappers emit no tags, and `slot=` never reaches the output.
	html.as_str()
		.xpect_contains("<header><h1>Title</h1></header>")
		.xpect_contains("<p>Body</p>")
		.xnot()
		.xpect_contains("slot=");
}

#[beet_core::test]
fn children_prop_preserves_order_and_multiplicity() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! {
			<Panel><p>"one"</p><p>"two"</p><p>"three"</p></Panel>
		})
		.unwrap()
		.id();
	let html = render_html(&mut world, root);
	html.find("one").unwrap().xpect_less_than(html.find("two").unwrap());
	html.find("two").unwrap().xpect_less_than(html.find("three").unwrap());
}

#[beet_core::test]
fn unset_prop_renders_empty() {
	// no caller content: both props default to empty, leaving bare containers.
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Panel/> }).unwrap().id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("<section>")
		.xnot()
		.xpect_contains("slot");
}

#[beet_core::test]
fn slot_fallback_renders_when_unset() {
	// no caller content: each `<slot>`'s own children are the fallback.
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <FallbackPanel/> }).unwrap().id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("<header>Default Title</header>")
		.xpect_contains("<div>Default Body</div>");
}

#[beet_core::test]
fn slot_fallback_overridden_by_caller() {
	// caller content replaces the fallback in both the named and default slot.
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! {
			<FallbackPanel>
				<h1 slot="header">"Real Title"</h1>
				<p>"Real Body"</p>
			</FallbackPanel>
		})
		.unwrap()
		.id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("<header><h1>Real Title</h1></header>")
		.xpect_contains("<p>Real Body</p>")
		.xnot()
		.xpect_contains("Default");
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
