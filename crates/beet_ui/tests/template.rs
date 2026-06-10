//! Tests for the `#[template]` and `#[template(system)]` macros: prop grammar
//! (required/default/option/into), in-process body lowering, slots
//! (named/default/fallback/transfer), system world-reads, and events. This is
//! part of the Task 3 gate.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::classes;
use beet_ui::prelude::*;

/// A spawn-capable template world.
fn world() -> World { ui_world() }

/// Render `root` to an HTML string.
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}

/// Values of an entity's value-bearing descendants, depth-first.
fn descendant_values(world: &World, root: Entity) -> Vec<String> {
	let mut out = Vec::new();
	let mut stack = vec![root];
	while let Some(entity) = stack.pop() {
		if let Some(value) = world.entity(entity).get::<Value>() {
			if let Ok(text) = value.as_str() {
				out.push(text.to_string());
			}
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			stack.extend(children.iter().rev());
		}
	}
	out
}

// ---- prop grammar -----------------------------------------------------------

#[template]
fn Card(#[prop(into)] title: String) -> impl Bundle {
	rsx! { <div class="card">{title}</div> }
}

#[beet_core::test]
fn into_prop_flows_to_body() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <Card title="Hi"/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
	descendant_values(&world, root).xpect_eq(vec!["Hi".to_string()]);
}

#[template]
fn Tag(
	#[prop(default = "tag")] kind: String,
	label: Option<String>,
) -> impl Bundle {
	let label = label.unwrap_or_default();
	rsx! { <span class={kind}>{label}</span> }
}

#[beet_core::test]
fn default_and_option_props() {
	let mut world = world();
	// omitted props fall back: `kind` -> "tag", `label` -> None (empty child).
	let root = world.spawn_template(rsx! { <Tag/> }).unwrap().id();
	world.with_state::<AttributeQuery, _>(|query| {
		query
			.find(root, "class")
			.unwrap()
			.1
			.as_str()
			.unwrap()
			.xpect_eq("tag");
	});

	// supplied props flow through.
	let root = world.spawn_template(rsx! { <Tag kind="x" label="hi"/> }).unwrap().id();
	world.with_state::<AttributeQuery, _>(|query| {
		query.find(root, "class").unwrap().1.as_str().unwrap().xpect_eq("x");
	});
	descendant_values(&world, root).contains(&"hi".to_string()).xpect_true();
}

#[derive(Default, Clone, PartialEq, Reflect)]
enum Variant {
	#[default]
	Filled,
	Error,
}

#[template]
fn Badge(#[prop(required)] variant: Variant) -> impl Bundle {
	let class = match variant {
		Variant::Filled => "filled",
		Variant::Error => "error",
	};
	rsx! { <span class={class}/> }
}

#[beet_core::test]
fn required_prop_supplied_resolves() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <Badge variant=Variant::Error/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("span");
	world.with_state::<AttributeQuery, _>(|query| {
		query.find(root, "class").unwrap().1.as_str().unwrap().xpect_eq("error");
	});
}

#[beet_core::test]
fn missing_required_prop_surfaces_error() {
	let mut world = world();
	let error = Store::new(None);
	let err = error.clone();
	world.add_observer(move |ev: On<LoadTemplate>| err.set(Some(ev.is_error)));

	// `<Badge/>` leaves `variant` unset: a graceful TemplateError, never a panic.
	let root = world.spawn_template(rsx! { <Badge/> }).unwrap().id();
	error.get().xpect_eq(Some(true));
	let template_error = world.entity(root).get::<TemplateError>().unwrap();
	template_error
		.error
		.to_string()
		.xpect_contains("missing required props")
		.xpect_contains("variant");
}

// ---- system templates -------------------------------------------------------

#[derive(Resource, Clone)]
struct AppTitle(String);

#[template(system)]
fn AppInfo(config: Res<AppTitle>) -> impl Bundle {
	let title = config.0.clone();
	rsx! { <article>{title}</article> }
}

#[beet_core::test]
fn system_template_reads_resource() {
	let mut world = world();
	world.insert_resource(AppTitle("beet".into()));
	let root = world.spawn_template(rsx! { <AppInfo/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("article");
	descendant_values(&world, root).xpect_eq(vec!["beet".to_string()]);
}

#[template(system)]
fn Panel(#[prop] role: String, config: Res<AppTitle>) -> impl Bundle {
	let title = config.0.clone();
	rsx! { <section class={role}>{title}</section> }
}

#[beet_core::test]
fn system_template_mixes_props_and_params() {
	let mut world = world();
	world.insert_resource(AppTitle("beet".into()));
	let root = world.spawn_template(rsx! { <Panel role="main"/> }).unwrap().id();
	world.with_state::<AttributeQuery, _>(|query| {
		query.find(root, "class").unwrap().1.as_str().unwrap().xpect_eq("main");
	});
	descendant_values(&world, root).xpect_eq(vec!["beet".to_string()]);
}

// ---- events -----------------------------------------------------------------

#[derive(EntityEvent)]
struct Bump(Entity);
#[derive(Resource, Default)]
struct Count(u32);

#[template]
fn Counter(#[prop(into)] label: String) -> impl Bundle {
	rsx! {
		<button
			{Classes::new(["btn", "btn-counter"])}
			onclick={|_: On<Bump>, mut count: ResMut<Count>| count.0 += 1}
		>
			{label}
		</button>
	}
}

#[beet_core::test]
fn template_event_attaches_observer() {
	let mut world = world();
	world.init_resource::<Count>();
	let root = world.spawn_template(rsx! { <Counter label="Bump"/> }).unwrap().id();

	world.resource::<Count>().0.xpect_eq(0);
	world.trigger(Bump(root));
	world.trigger(Bump(root));
	world.resource::<Count>().0.xpect_eq(2);
	// the semantic classes attached, not a `class=` string.
	world.entity(root).get::<Classes>().unwrap().contains_selector("btn").xpect_true();
}

// ---- slots ------------------------------------------------------------------

#[template]
fn LayoutPanel() -> impl Bundle {
	rsx! {
		<section>
			<header><Slot name="header"/></header>
			<div><Slot/></div>
		</section>
	}
}

#[beet_core::test]
fn named_and_default_slots() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! {
			<LayoutPanel>
				<h1 slot="header">"Title"</h1>
				<p>"Body"</p>
			</LayoutPanel>
		})
		.unwrap().id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("<header><h1>Title</h1></header>")
		.xpect_contains("<p>Body</p>")
		.xnot()
		.xpect_contains("slot=");
}

#[template]
fn FallbackPanel() -> impl Bundle {
	rsx! {
		<section>
			<header><Slot name="header">"Default Title"</Slot></header>
			<div><Slot>"Default Body"</Slot></div>
		</section>
	}
}

#[beet_core::test]
fn slot_fallback_renders_when_unset() {
	let mut world = world();
	let root = world.spawn_template(rsx! { <FallbackPanel/> }).unwrap().id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("Default Title")
		.xpect_contains("Default Body");
}

#[beet_core::test]
fn slot_fallback_overridden_by_caller() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! {
			<FallbackPanel>
				<h1 slot="header">"Real Title"</h1>
				<p>"Real Body"</p>
			</FallbackPanel>
		})
		.unwrap().id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("<header><h1>Real Title</h1></header>")
		.xpect_contains("<p>Real Body</p>")
		.xnot()
		.xpect_contains("Default");
}

#[beet_core::test]
fn slot_children_preserve_order() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! {
			<LayoutPanel><p>"one"</p><p>"two"</p><p>"three"</p></LayoutPanel>
		})
		.unwrap().id();
	let html = render_html(&mut world, root);
	html.find("one").unwrap().xpect_less_than(html.find("two").unwrap());
	html.find("two").unwrap().xpect_less_than(html.find("three").unwrap());
}

// nested templates: a widget that exposes a default slot, used as content of
// another widget's named slot.
#[template]
fn Span() -> impl Bundle {
	rsx! { <span><Slot/></span> }
}

#[beet_core::test]
fn nested_template_in_named_slot() {
	let mut world = world();
	let root = world
		.spawn_template(rsx! {
			<LayoutPanel>
				<Span slot="header">"Title"</Span>
				<p>"Body"</p>
			</LayoutPanel>
		})
		.unwrap().id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("<header><span>Title</span></header>")
		.xpect_contains("<p>Body</p>");
}

// reference the imported `classes` so the import is exercised.
#[beet_core::test]
fn classes_constant_is_a_class_name() {
	let _ = classes::BTN;
}
