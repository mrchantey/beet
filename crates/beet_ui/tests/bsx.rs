//! Tests for the BSX parser, its lowering to a scene body and the body's spawn:
//! literals with type-inferred reflect, `@` bindings, `$` references, spreads,
//! `bx:` directives, slots, and events, asserted on the spawned tree. A `.bsx`
//! snippet must spawn the same tree the equivalent `rsx!` builds.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;
use beet_ui::prelude::*;
use bevy::reflect::GetTypeRegistration;

/// A spawn-capable template world.
fn world() -> World { world_ext::ui_world() }

/// Register a reflect type into the world's [`AppTypeRegistry`].
fn register<T: GetTypeRegistration>(world: &mut World) {
	world
		.resource_mut::<AppTypeRegistry>()
		.write()
		.register::<T>();
}

/// Parse a `.bsx` source and return its root.
fn spawn_bsx(world: &mut World, source: &str) -> Entity {
	spawn_bsx_under(world, None, source)
}

/// Parse a `.bsx` source under `parent` (so the document machinery links before
/// the tree is built), returning its root.
fn spawn_bsx_under(
	world: &mut World,
	parent: Option<Entity>,
	source: &str,
) -> Entity {
	let bytes = MediaBytes::new_bsx(source);
	let mut entity = match parent {
		Some(parent) => world.spawn(ChildOf(parent)),
		None => world.spawn_empty(),
	};
	DocParsers::parse(ParseContext::new(&mut entity, &bytes)).unwrap();
	entity.id()
}

/// The [`world`] app activating instances, with a store of `documents` on an
/// entity its instances resolve from.
async fn package(documents: &[(&str, &str)]) -> (World, Entity) {
	let mut app = world_ext::ui_app();
	app.install_types::<AsyncPlugin>()
		.install_types::<ScenePlugin>();
	let mut world = core::mem::take(app.world_mut());
	let store = BlobStore::temp();
	for (path, source) in documents {
		store
			.insert(&RelPath::new(path), source.to_string())
			.await
			.unwrap();
	}
	let package = world.spawn(PackageStore::new(store)).id();
	(world, package)
}

/// Spawn `source` under `package` and settle the instances in it.
async fn spawn_in(world: &mut World, package: Entity, source: &str) -> Entity {
	let root = spawn_bsx_under(world, Some(package), source);
	AsyncRunner::settle_async_tasks(world).await;
	for _ in 0..3 {
		world.update_local();
	}
	root
}

/// Render `root` to an HTML string.
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(
			world,
			root,
			&RequestParts::default(),
		))
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

// ---- basic markup -----------------------------------------------------------

#[beet_core::test]
fn element_and_text() {
	let mut world = world();
	let root = spawn_bsx(&mut world, "<div>hello</div>");
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("div");
	descendant_values(&world, root).xpect_eq(vec!["hello".to_string()]);
}

#[beet_core::test]
fn nested_elements() {
	let mut world = world();
	let root = spawn_bsx(&mut world, "<div><span>inner</span></div>");
	let span = world.entity(root).get::<Children>().unwrap()[0];
	world
		.entity(span)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("span");
}

#[beet_core::test]
fn attributes_become_attribute_entities() {
	let mut world = world();
	let root = spawn_bsx(&mut world, "<div class=\"card\" id=\"main\"/>");
	world.with_state::<AttributeQuery, _>(|query| {
		query
			.find(root, "class")
			.unwrap()
			.1
			.as_str()
			.unwrap()
			.xpect_eq("card");
		query
			.find(root, "id")
			.unwrap()
			.1
			.as_str()
			.unwrap()
			.xpect_eq("main");
	});
}

#[beet_core::test]
fn bsx_and_rsx_match() {
	// the same markup through both front-ends must render identically.
	let mut world_a = world();
	let bsx_root =
		spawn_bsx(&mut world_a, "<div class=\"card\"><span>hi</span></div>");
	let bsx_html = render_html(&mut world_a, bsx_root);

	let mut world_b = world();
	let rsx_root = world_b
		.spawn_template(rsx! { <div class="card"><span>"hi"</span></div> })
		.unwrap()
		.id();
	let rsx_html = render_html(&mut world_b, rsx_root);

	bsx_html.xpect_eq(rsx_html);
}

// ---- literals + type-inferred reflect ---------------------------------------

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Sized {
	width: f32,
	height: f32,
}

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Aligned {
	align: Align,
}

#[derive(Reflect, Default, Clone, PartialEq, Debug)]
enum Align {
	#[default]
	Start,
	Center,
	End,
}

#[beet_core::test]
fn struct_literal_infers_field_types() {
	let mut world = world();
	register::<Sized>(&mut world);
	// `0` coerces to `0.0f32`, `4` to `4.0f32`, against the `Sized` fields.
	let root = spawn_bsx(&mut world, "<entity {Sized{width:8,height:4}}/>");
	world
		.entity(root)
		.get::<Sized>()
		.cloned()
		.unwrap()
		.xpect_eq(Sized {
			width: 8.0,
			height: 4.0,
		});
}

#[beet_core::test]
fn enum_literal_infers_variant() {
	let mut world = world();
	register::<Aligned>(&mut world);
	let root = spawn_bsx(&mut world, "<entity {Aligned{align:Center}}/>");
	world
		.entity(root)
		.get::<Aligned>()
		.cloned()
		.unwrap()
		.xpect_eq(Aligned {
			align: Align::Center,
		});
}

// ---- @doc bindings -----------------------------------------------------------

#[beet_core::test]
fn field_ref_binds_document() {
	let mut world = world();
	let doc = world.spawn(Document::new(value!({ "count": 7 }))).id();
	let root =
		spawn_bsx_under(&mut world, Some(doc), "<span>{@doc:count}</span>");
	world.update_local();
	// the text child holds a FieldRef bound to `count`, synced to 7.
	let text = world.entity(root).get::<Children>().unwrap()[0];
	world.entity(text).get::<FieldRef>().is_some().xpect_true();
	world
		.entity(text)
		.get::<Value>()
		.unwrap()
		.as_i64()
		.unwrap()
		.xpect_eq(7);
}

#[beet_core::test]
fn field_ref_init_seeds_when_missing() {
	let mut world = world();
	let doc = world.spawn(Document::new(value!({}))).id();
	let _root =
		spawn_bsx_under(&mut world, Some(doc), "<span>{@doc:count=5}</span>");
	world.update_local();
	world.update_local();
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[FieldSegment::key("count")])
		.unwrap()
		.xpect_eq(5);
}

// ---- $entity references (incl forward) --------------------------------------

#[beet_core::test]
fn entity_ref_resolves_forward() {
	let mut world = world();
	register::<Linked>(&mut world);
	// `$target` is used before the `bx:ref="target"` entity is built.
	let root = spawn_bsx(
		&mut world,
		"<div><a {Linked{to:$target}}/><b bx:ref=\"target\"/></div>",
	);
	let children = world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.iter()
		.collect::<Vec<_>>();
	let linked_holder = children[0];
	let target = children[1];
	world
		.entity(linked_holder)
		.get::<Linked>()
		.unwrap()
		.to
		.xpect_eq(target);
}

#[derive(Component, Reflect, MapEntities, Clone, Debug)]
#[reflect(Component, MapEntities, Default)]
struct Linked {
	#[entities]
	to: Entity,
}

impl Default for Linked {
	fn default() -> Self {
		Self {
			to: Entity::PLACEHOLDER,
		}
	}
}

// ---- enum-variant entity refs -----------------------------------------------

/// An enum component with an `Entity` variant, mirroring `SteerTarget`, to
/// exercise a markup ref resolving directly into a variant.
#[derive(Component, Reflect, MapEntities, Default, Clone, Debug, PartialEq)]
#[reflect(Component, MapEntities, Default)]
enum Aim {
	#[default]
	Idle,
	At(#[entities] Entity),
}

/// A `{EnumType::Variant($ref)}` spread resolves the ref into the real variant,
/// eg `{SteerTarget::Entity($cheese)}`, so markup can target an enum variant
/// directly (replacing the `SteerTargetEntity` on-insert adapter).
#[beet_core::test]
fn enum_variant_resolves_entity_ref() {
	let mut world = world();
	register::<Aim>(&mut world);
	let root = spawn_bsx(
		&mut world,
		"<div><a {Aim::At($target)}/><b bx:ref=\"target\"/></div>",
	);
	let children = world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.iter()
		.collect::<Vec<_>>();
	let target = children[1];
	world
		.entity(children[0])
		.get::<Aim>()
		.unwrap()
		.clone()
		.xpect_eq(Aim::At(target));
}

/// A tuple-struct component with an `Entity` field, mirroring `ActionOf`.
#[derive(Component, Reflect, MapEntities, Clone, Debug, PartialEq)]
#[reflect(Component, MapEntities, Default)]
struct Bound(#[entities] Entity);

impl Default for Bound {
	fn default() -> Self { Self(Entity::PLACEHOLDER) }
}

/// A `{TupleStruct($ref)}` spread resolves the ref directly into the field, eg
/// `{ActionOf($fox)}`, no newtype wrapper needed.
#[beet_core::test]
fn tuple_struct_resolves_entity_ref() {
	let mut world = world();
	register::<Bound>(&mut world);
	let root = spawn_bsx(
		&mut world,
		"<div><a {Bound($target)}/><b bx:ref=\"target\"/></div>",
	);
	let children = world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.iter()
		.collect::<Vec<_>>();
	let target = children[1];
	world
		.entity(children[0])
		.get::<Bound>()
		.unwrap()
		.0
		.xpect_eq(target);
}

// ---- spreads ----------------------------------------------------------------

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Marker;

#[beet_core::test]
fn spread_inserts_components() {
	let mut world = world();
	register::<Marker>(&mut world);
	register::<Sized>(&mut world);
	let root =
		spawn_bsx(&mut world, "<entity {(Marker, Sized{width:2,height:2})}/>");
	world.entity(root).contains::<Marker>().xpect_true();
	world
		.entity(root)
		.get::<Sized>()
		.unwrap()
		.width
		.xpect_eq(2.0);
}

#[beet_core::test]
fn spread_on_component_tag() {
	// a spread stacks components onto an uppercase tag's entity, eg middleware
	// on a router: `<Router {(RequestLogger, ..)}>`.
	let mut world = world();
	register::<Marker>(&mut world);
	register::<Sized>(&mut world);
	let root = spawn_bsx(&mut world, "<Sized width=1 height=1 {Marker}/>");
	world.entity(root).contains::<Marker>().xpect_true();
	world
		.entity(root)
		.get::<Sized>()
		.unwrap()
		.width
		.xpect_eq(1.0);
}

#[beet_core::test]
fn fragment_hosts_bundle_without_element() {
	let mut world = world();
	register::<Marker>(&mut world);
	// the fragment carries the spread and hosts children, but emits no `Element`.
	let root = spawn_bsx(&mut world, "<Fragment {Marker}><span/></Fragment>");
	world.entity(root).contains::<Marker>().xpect_true();
	world.entity(root).contains::<Element>().xpect_false();
	world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.len()
		.xpect_eq(1);
}

/// A second component sharing the short path `Transform`, forcing the same
/// ambiguity the real CSS `beet_ui::style::values::layout::Transform` does.
mod css_dup {
	use beet_core::prelude::*;
	#[derive(Component, Reflect, Default)]
	#[reflect(Component, Default)]
	pub struct Transform;
}

/// A fully-qualified tag (the registered reflect type path) resolves the bevy
/// `Transform` from markup despite the ambiguous bare short path. Neither
/// candidate is a template, so the bare path resolves to neither: an inert
/// entity recording the name, never a silent wrong pick.
#[beet_core::test]
fn qualified_transform_tag_disambiguates() {
	use bevy::reflect::Typed;
	let mut world = world();
	register::<Transform>(&mut world);
	register::<css_dup::Transform>(&mut world);
	// the fully-qualified reflect path resolves the bevy transform, not the CSS one
	let path = Transform::type_info().type_path();
	let root = spawn_bsx(&mut world, &format!("<{path}/>"));
	world.entity(root).contains::<Transform>().xpect_true();
	world
		.entity(root)
		.contains::<css_dup::Transform>()
		.xpect_false();
	// the bare, ambiguous short path picks neither.
	let root = spawn_bsx(&mut world, "<Transform/>");
	world.entity(root).contains::<Transform>().xpect_false();
	world
		.entity(root)
		.contains::<css_dup::Transform>()
		.xpect_false();
	world
		.entity(root)
		.get::<Unregistered>()
		.unwrap()
		.contains_key("Transform")
		.xpect_true();
}

// ---- bx:scope ---------------------------------------------------------------

#[beet_core::test]
fn scope_prefixes_descendant_fields() {
	let mut world = world();
	let doc = world
		.spawn(Document::new(value!({ "user": { "name": "Alice" } })))
		.id();
	let root = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<div bx:scope=\"user\"><span>{@doc:name}</span></div>",
	);
	world.update_local();
	world.entity(root).contains::<DocumentScope>().xpect_true();
	let span = world.entity(root).get::<Children>().unwrap()[0];
	let text = world.entity(span).get::<Children>().unwrap()[0];
	world
		.entity(text)
		.get::<Value>()
		.unwrap()
		.as_str()
		.unwrap()
		.xpect_eq("Alice");
}

// ---- bx:for + bx:key --------------------------------------------------------

#[beet_core::test]
fn reactive_children_per_item() {
	let mut world = world();
	let doc = world
		.spawn(Document::new(value!({ "items": ["a", "b", "c"] })))
		.id();
	let list = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<ul bx:for=\"items\" bx:key=\"i\"><li>x</li></ul>",
	);
	world.update_local();
	world.update_local();
	// one row per item, each a clone of the held item template, whose body
	// is the element's children
	let rows = world
		.entity(list)
		.get::<Children>()
		.unwrap()
		.iter()
		.filter(|child| world.entity(*child).contains::<ReactiveChild>())
		.collect::<Vec<_>>();
	rows.len().xpect_eq(3);
	for row in rows {
		let item = world.entity(row).get::<Children>().unwrap()[0];
		world.get::<Element>(item).unwrap().tag().xpect_eq("li");
	}
}

/// A list saves as its generator and held item template, never its rows, so a
/// reload regenerates the rows rather than freezing them.
#[cfg(feature = "json")]
#[beet_core::test]
fn reactive_children_survive_a_save_and_a_reload() {
	let rows = |world: &World, list: Entity| {
		world
			.entity(list)
			.get::<Children>()
			.unwrap()
			.iter()
			.filter(|child| world.entity(*child).contains::<ReactiveChild>())
			.count()
	};
	let mut world = world();
	let doc = world
		.spawn(Document::new(value!({ "items": ["a", "b"] })))
		.id();
	let list = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<ul bx:for=\"items\"><li>x</li></ul>",
	);
	world.update_local();
	rows(&world, list).xpect_eq(2);
	let bytes = TemplateSaver::new()
		.save_roots(&mut world, MediaType::Json, [list])
		.unwrap();
	bytes
		.as_utf8()
		.unwrap()
		.xref()
		.xpect_contains("ReactiveChildren")
		.xpect_contains("Prefab")
		.xpect_contains("template")
		.xnot()
		.xpect_contains("Held");

	let mut world = self::world();
	let doc = world
		.spawn(Document::new(value!({ "items": ["a", "b", "c"] })))
		.id();
	let list = TemplateLoader::new(&mut world)
		.with_entity(doc)
		.load(&bytes)
		.unwrap()[0];
	world.update_local();
	rows(&world, list).xpect_eq(3);
}

/// The rows a [`ReactiveChildren`] generated under `list`, in order.
fn reactive_rows(world: &World, list: Entity) -> Vec<Entity> {
	world
		.entity(list)
		.get::<Children>()
		.unwrap()
		.iter()
		.filter(|child| world.entity(*child).contains::<ReactiveChild>())
		.collect()
}

/// A keyed list authored with its `<template>` and one written with the
/// `bx:for` that is sugar for it build the same rows, each the template's
/// content binding its own item, and an append reuses every existing row.
#[beet_core::test]
fn a_keyed_list_reuses_its_rows_authored_or_sugared() {
	for markup in [
		r#"<ul {(ReactiveChildren{key:"name"}, @doc:entries)}><template><li>{@doc:name}</li></template></ul>"#,
		r#"<ul bx:for="entries" bx:key="name"><li>{@doc:name}</li></ul>"#,
	] {
		let mut world = world();
		let doc = world
			.spawn(Document::new(value!({
				"entries": [{ "name": "ada" }, { "name": "bob" }]
			})))
			.id();
		let list = spawn_bsx_under(&mut world, Some(doc), markup);
		world.update_local();
		let before = reactive_rows(&world, list);
		before.len().xpect_eq(2);
		world
			.entity_mut(doc)
			.get_mut::<Document>()
			.unwrap()
			.0
			.get_mut("entries")
			.unwrap()
			.as_list_mut()
			.unwrap()
			.push(value!({ "name": "cleo" }));
		world.update_local();
		let after = reactive_rows(&world, list);
		after.len().xpect_eq(3);
		after[..2].to_vec().xpect_eq(before);
		render_html(&mut world, list)
			.xref()
			.xpect_contains("<li>ada</li><li>bob</li><li>cleo</li>")
			.xnot()
			.xpect_contains("template");
	}
}

// ---- scripts and what renders -------------------------------------------------

/// A script beet runs is behaviour, neither rendered nor any part of its
/// parent's text, where a bare `<script>` is page content for the browser.
#[beet_core::test]
fn a_run_script_is_not_rendered() {
	let mut world = world();
	let root = spawn_bsx(
		&mut world,
		r#"<div><p>hi<script bx:run>secret()</script></p><script>page()</script></div>"#,
	);
	render_html(&mut world, root)
		.xref()
		.xpect_contains("<p>hi</p>")
		.xpect_contains("page()")
		.xnot()
		.xpect_contains("secret");
}

/// An authored `Disabled` entity and a held `<template>` are both left out of
/// the render walk, by Bevy's default filters rather than by name.
#[beet_core::test]
fn disabled_and_held_entities_are_not_rendered() {
	let mut world = world();
	let root = spawn_bsx(
		&mut world,
		r#"<div><p {Disabled}>off</p><template><p>held</p></template><p>on</p></div>"#,
	);
	world.update_local();
	render_html(&mut world, root)
		.xref()
		.xpect_contains("on")
		.xnot()
		.xpect_contains("off")
		.xnot()
		.xpect_contains("held");
}

// ---- props ------------------------------------------------------------------

/// Read the local [`Value`] of `entity`.
fn read_value(world: &World, entity: Entity) -> Value {
	world.entity(entity).get::<Value>().unwrap().clone()
}

/// Each use's props are its own, whatever binds them: a literal, a document,
/// a resource or the props of the use around it.
#[beet_core::test]
async fn props_bind_every_source() {
	let (mut world, package) = package(&[
		("Card.bsx", "<section>{@prop:title}</section>"),
		(
			"Outer.bsx",
			r#"<div><Instance document="./Card" props={{title:@prop:title}}/></div>"#,
		),
	])
	.await;
	register::<Theme>(&mut world);
	world.insert_resource(Theme {
		contrast: 5,
		..default()
	});
	world
		.entity_mut(package)
		.insert(Document::new(value!({ "name": "Alice" })));
	let mut render = async |world: &mut World, source: &str| {
		let root = spawn_in(world, package, source).await;
		(root, render_html(world, root))
	};
	// a user scope above the use never reaches its props
	let (_, html) = render(
		&mut world,
		r#"<div bx:scope="user"><Instance document="Card" props={{title:"hi"}}/></div>"#,
	)
	.await;
	html.xpect_contains("<section>hi</section>");
	let (_, html) = render(
		&mut world,
		r#"<Instance document="Outer" props={{title:"hello"}}/>"#,
	)
	.await;
	html.xpect_contains("<section>hello</section>");
	let (root, html) = render(
		&mut world,
		r#"<Instance document="Card" props={{title:@res:Theme.contrast}}/>"#,
	)
	.await;
	html.xpect_contains("<section>5</section>");
	world.resource_mut::<Theme>().contrast = 9;
	for _ in 0..3 {
		world.update_local();
	}
	render_html(&mut world, root).xpect_contains("<section>9</section>");
	let (root, html) = render(
		&mut world,
		r#"<Instance document="Card" props={{title:@doc:name}}/>"#,
	)
	.await;
	html.xpect_contains("<section>Alice</section>");
	world.get_mut::<Document>(package).unwrap().0 = value!({ "name": "Bob" });
	for _ in 0..3 {
		world.update_local();
	}
	render_html(&mut world, root).xpect_contains("<section>Bob</section>");
}

/// A prop never shadows the document a body binding reads, the use site being
/// no user document.
#[beet_core::test]
async fn a_document_binding_skips_the_props() {
	let (mut world, package) =
		package(&[("Card.bsx", "<section>{@doc:name}</section>")]).await;
	world
		.entity_mut(package)
		.insert(Document::new(value!({ "name": "Alice" })));
	let root = spawn_in(
		&mut world,
		package,
		r#"<Instance document="Card" props={{name:"shadow"}}/>"#,
	)
	.await;
	render_html(&mut world, root).xpect_contains("<section>Alice</section>");
}

// ---- slots ------------------------------------------------------------------

/// A single `<Fragment slot="head">` carries its children into the slot, the
/// markup twin of each declaring it, leaving no wrapper of its own.
#[beet_core::test]
async fn fragment_forwards_children_into_slot() {
	let (mut world, package) = package(&[(
		"Host.bsx",
		"<html><head><slot name=\"head\"/></head><body><slot/></body></html>",
	)])
	.await;
	let root = spawn_in(
		&mut world,
		package,
		r#"<Instance document="Host"><Fragment slot="head"><meta/><link/><style/></Fragment><p>body</p></Instance>"#,
	)
	.await;
	let head = render_html(&mut world, root)
		.split("</head>")
		.next()
		.unwrap()
		.to_string();
	head.clone().xpect_contains("<meta");
	head.clone().xpect_contains("<link");
	head.clone().xpect_contains("<style");
	head.xnot().xpect_contains("<p>body</p>");
}

// ---- events -----------------------------------------------------------------

/// Fire a pointer-down on `entity` and drive the world until the event script's
/// evaluation has settled: the script is a task, and each `world` call it makes
/// is one more round trip through the async sync point.
#[cfg(feature = "scripting")]
async fn click(world: &mut World, entity: Entity) {
	let pointer = world.spawn_empty().id();
	world
		.entity_mut(entity)
		.trigger(move |target| PointerDown { target, pointer });
	AsyncRunner::settle_async_tasks(world).await;
}

/// A `bx:onclick` runs its authored script against the live world, and the script
/// is the whole vocabulary: nothing in rust knows what "increment" means.
#[cfg(feature = "scripting")]
#[beet_core::test]
async fn click_runs_its_script() {
	let mut world = world();
	let doc = world.spawn(Document::new(value!({ "count": 0 }))).id();
	let button = spawn_bsx_under(
		&mut world,
		Some(doc),
		r#"<button bx:onclick="target.with_field('count', count => count + 1)">+</button>"#,
	);
	world.update_local();
	// the host carries no mirror: the script writes the document, not a local Value.
	world.entity(button).contains::<FieldRef>().xpect_false();
	world.entity(button).contains::<Value>().xpect_false();
	click(&mut world, button).await;
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[FieldSegment::key("count")])
		.unwrap()
		.xpect_eq(1);
}

/// The counter page shape: a `bx:scope`, an initializing text binding, and an
/// event script mutating the same scoped field, with no pre-existing document.
/// Regressions covered: the scoped init path corrupting the document
/// (`{counter: 0}` instead of `{counter: {count: 0}}`), and the event host
/// carrying no mirror that could leak into the button's rendered label. The
/// click drives the end-to-end script -> scoped-document -> display-binding path.
#[cfg(feature = "scripting")]
#[beet_core::test]
async fn scoped_counter_page() {
	let mut world = world();
	let root = spawn_bsx(
		&mut world,
		r#"<article bx:scope="counter"><p>clicked {@doc:count=0} times</p><button bx:onclick="target.with_field('count', count => count + 1)">More</button></article>"#,
	);
	// settle the init -> document chain
	world.update_local();
	world.update_local();

	// the scoped init created the nested field, preserving the init value
	let (_, doc) = world.query::<(Entity, &Document)>().single(&world).unwrap();
	doc.get_field::<u64>(&[
		FieldSegment::key("counter"),
		FieldSegment::key("count"),
	])
	.unwrap()
	.xpect_eq(0);

	// the button carries no mirror, so nothing leaks into its label
	render_html(&mut world, root)
		.xpect_contains("clicked 0 times")
		.xpect_contains("<button>More</button>");

	// click the button: the script resolves the scoped field and writes the document
	let button = world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.iter()
		.find(|child| {
			world
				.entity(*child)
				.get::<Element>()
				.is_some_and(|el| el.tag() == "button")
		})
		.unwrap();
	click(&mut world, button).await;
	world.update_local();

	// the scoped field incremented, and the display binding refreshes to match
	let doc = world
		.query::<(Entity, &Document)>()
		.single(&world)
		.unwrap()
		.0;
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[
			FieldSegment::key("counter"),
			FieldSegment::key("count"),
		])
		.unwrap()
		.xpect_eq(1);
	render_html(&mut world, root).xpect_contains("clicked 1 times");
}

// ---- live TUI counter (after the `beetmania` no-code example) ----------------

/// A trimmed `examples/beetmania/routes/counter.bsx`, the no-code example: a
/// scoped document, a display binding, and two event scripts mutating it.
#[cfg(all(feature = "tui", feature = "scripting"))]
const COUNTER_BSX: &str = r#"<article bx:scope="counter">
	<Instance document="widgets/Card.bsx" props={{title:"Counter"}}>
		<p>You have clicked {@doc:count=0} times.</p>
		<button bx:onclick="target.with_field('count', count => count + 1)">More</button>
		<button bx:onclick="target.with_field('count', count => count - 1)">Less</button>
	</Instance>
</article>"#;

/// Drive `counter.bsx` through the real live-charcell stack (the terminal target):
/// a click runs the script, document-sync fans the change to the display binding,
/// and the charcell renderer repaints. Asserts the *rendered frame text* changes,
/// not just the document, exercising the click -> repaint path end to end.
#[cfg(all(feature = "tui", feature = "scripting"))]
#[beet_core::test]
async fn counter_bsx_repaints_in_live_tui() {
	use bevy::math::UVec2;

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, CharcellTuiPlugin))
		.install_types::<AsyncPlugin>()
		.install_types::<ScenePlugin>();
	// the site-local card the counter instantiates
	let store = BlobStore::temp();
	store
		.insert(
			&RelPath::new("widgets/Card.bsx"),
			"<section><h2>{@prop:title}</h2><slot/></section>",
		)
		.await
		.unwrap();
	// the host buffer the charcell pipeline paints into, plus a channel terminal
	// so the run is headless and deterministic.
	let (channel, terminal) = ChannelTerminal::new(TerminalConfig::default());
	let host = app
		.world_mut()
		.spawn((
			channel,
			terminal,
			DoubleBuffer::new(UVec2::new(48, 12)),
			PackageStore::new(store),
		))
		.id();
	app.update();
	// parse the counter markup as the host's content tree.
	let bytes = MediaBytes::new_bsx(COUNTER_BSX);
	DocParsers::parse(ParseContext::new(
		&mut app.world_mut().entity_mut(host),
		&bytes,
	))
	.unwrap();

	// step until the scoped `@doc:count=0` init reaches the rendered frame.
	let frame = step_until(&mut app, host, "clicked 0 times").await;
	frame.xpect_contains("Counter");

	// click "More": fire a `PointerDown` on the increment button, like the hit-test
	// does for a real cursor press.
	let more = find_button(&mut app, host, "More");
	tui_click(&mut app, more);
	// the script wrote the document; the next frames sync the binding and repaint.
	step_until(&mut app, host, "clicked 1 times").await;

	// click "Less": the decrement script walks back to the same scoped field.
	let less = find_button(&mut app, host, "Less");
	tui_click(&mut app, less);
	step_until(&mut app, host, "clicked 0 times").await;
}

/// Trigger a `PointerDown` on `entity`, then flush so the queued command runs.
#[cfg(all(feature = "tui", feature = "scripting"))]
fn tui_click(app: &mut App, entity: Entity) {
	let pointer = app.world_mut().spawn_empty().id();
	app.world_mut()
		.entity_mut(entity)
		.trigger(move |target| PointerDown { target, pointer });
	app.world_mut().flush();
}

/// The `<button>` entity beneath `host` whose rendered text is `label`.
#[cfg(all(feature = "tui", feature = "scripting"))]
fn find_button(app: &mut App, host: Entity, label: &str) -> Entity {
	let label = label.to_string();
	app.world_mut()
		.run_system_once(move |elements: ElementQuery| {
			elements
				.iter_descendants_inclusive(host)
				.find(|view| {
					view.tag() == "button"
						&& view.inner_text.is_some_and(|(_, text)| {
							text.as_str().is_ok_and(|text| text == label)
						})
				})
				.map(|view| view.entity)
		})
		.unwrap()
		.expect("a button with that label")
}

/// Advance frames until the host's painted frame contains `needle`, returning it.
///
/// Async because a click's script is a task: each frame both runs the world and
/// lets the evaluation progress, so the paint that follows it is reached.
#[cfg(all(feature = "tui", feature = "scripting"))]
async fn step_until(app: &mut App, host: Entity, needle: &str) -> String {
	let frame = |app: &App| {
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.front_buffer()
			.render_plain()
	};
	for _ in 0..200 {
		AsyncRunner::step(app).await;
		let painted = frame(app);
		if painted.contains(needle) {
			return painted;
		}
	}
	panic!("frame never contained {needle:?}:\n{}", frame(app));
}

// ---- @ bindings ---------------------------------------------------------------

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Slider {
	value: i64,
}

#[derive(Resource, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Resource, Default)]
struct Theme {
	contrast: i64,
}

/// A world with a registered + inserted `Theme { contrast: 5 }`.
fn theme_world() -> World {
	let mut world = world();
	register::<Theme>(&mut world);
	world.insert_resource(Theme { contrast: 5 });
	world
}

/// The local [`Value`] of `entity` as an i64.
fn read_i64(world: &World, entity: Entity) -> i64 {
	read_value(world, entity).as_i64().unwrap()
}

#[beet_core::test]
fn binding_doc_text() {
	let mut world = world();
	let doc = world.spawn(Document::new(value!({ "count": 7 }))).id();
	let root =
		spawn_bsx_under(&mut world, Some(doc), "<span>{@doc:count}</span>");
	world.update_local();
	let text = world.entity(root).get::<Children>().unwrap()[0];
	world.entity(text).contains::<FieldRef>().xpect_true();
	read_i64(&world, text).xpect_eq(7);

	// reactive: a document change reaches the text
	world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
		value!({ "count": 8 });
	world.update_local();
	read_i64(&world, text).xpect_eq(8);
}

#[beet_core::test]
fn binding_doc_init_seeds_when_missing() {
	let mut world = world();
	let doc = world.spawn(Document::new(value!({}))).id();
	spawn_bsx_under(&mut world, Some(doc), "<span>{@doc:count=5}</span>");
	world.update_local();
	world.update_local();
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[FieldSegment::key("count")])
		.unwrap()
		.xpect_eq(5);
}

#[beet_core::test]
fn binding_doc_attribute_lowers_field_ref() {
	let mut world = world();
	let doc = world.spawn(Document::new(value!({ "name": "Ada" }))).id();
	let root =
		spawn_bsx_under(&mut world, Some(doc), "<input value=@doc:name/>");
	// the attribute entity carries the FieldRef, exactly the `#` lowering
	let attr = world
		.entity(root)
		.get::<Attributes>()
		.unwrap()
		.iter()
		.next()
		.unwrap();
	world
		.entity(attr)
		.get::<FieldRef>()
		.unwrap()
		.field_path
		.to_string()
		.xpect_eq("name".to_string());
}

#[beet_core::test]
fn binding_res_text() {
	let mut world = theme_world();
	let root = spawn_bsx(&mut world, "<span>{@res:Theme.contrast}</span>");
	world.update_local();
	let text = world.entity(root).get::<Children>().unwrap()[0];
	read_i64(&world, text).xpect_eq(5);

	// reactive: a resource change reaches the text
	world.resource_mut::<Theme>().contrast = 9;
	world.update_local();
	read_i64(&world, text).xpect_eq(9);
}

#[beet_core::test]
fn binding_res_attribute_renders() {
	let mut world = theme_world();
	let root = spawn_bsx(&mut world, "<input value=@res:Theme.contrast/>");
	world.update_local();
	render_html(&mut world, root).xpect_contains("value=\"5\"");

	world.resource_mut::<Theme>().contrast = 9;
	world.update_local();
	render_html(&mut world, root).xpect_contains("value=\"9\"");
}

// ---- resource declaration tags ------------------------------------------------

/// A two-field resource so a patch can prove the untouched field survives.
#[derive(Resource, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Resource, Default)]
struct SiteMeta {
	title: String,
	tagline: String,
}

#[beet_core::test]
fn resource_tag_patches_existing_resource() {
	let mut world = world();
	register::<SiteMeta>(&mut world);
	world.insert_resource(SiteMeta {
		title: "old".into(),
		tagline: "keep".into(),
	});
	let node = spawn_bsx(&mut world, "<SiteMeta title=\"new\"/>");
	// the named field patched, the missing field kept
	world.resource::<SiteMeta>().title.as_str().xpect_eq("new");
	world
		.resource::<SiteMeta>()
		.tagline
		.as_str()
		.xpect_eq("keep");
	// the tag produces no entity content
	world.entity(node).contains::<Element>().xpect_false();
	world.entity(node).contains::<Value>().xpect_false();
}

#[beet_core::test]
fn resource_tag_inserts_when_absent() {
	let mut world = world();
	register::<SiteMeta>(&mut world);
	spawn_bsx(&mut world, "<SiteMeta title=\"fresh\"/>");
	// inserted over the type's default
	world
		.resource::<SiteMeta>()
		.title
		.as_str()
		.xpect_eq("fresh");
	world.resource::<SiteMeta>().tagline.as_str().xpect_eq("");
}

/// The error lowering `source` refuses with.
fn build_error(world: &mut World, source: &str) -> String {
	SceneBody::parse_bsx(world, source).unwrap_err().to_string()
}

#[beet_core::test]
fn resource_tag_children_error() {
	let mut world = world();
	register::<SiteMeta>(&mut world);
	build_error(&mut world, "<SiteMeta title=\"x\"><span/></SiteMeta>")
		.xpect_contains("cannot have children");
}

#[beet_core::test]
fn resource_tag_binding_attr_error() {
	let mut world = world();
	register::<SiteMeta>(&mut world);
	build_error(&mut world, "<SiteMeta title=@doc:name/>")
		.xpect_contains("cannot declare a resource field");
}

#[beet_core::test]
fn binding_comp_attribute_targets_element() {
	let mut world = world();
	register::<Slider>(&mut world);
	let root = spawn_bsx(
		&mut world,
		"<input value=@comp:Slider.value {Slider{value:2}}/>",
	);
	world.update_local();
	render_html(&mut world, root).xpect_contains("value=\"2\"");

	// reactive: a component edit reaches the rendered attribute
	world.entity_mut(root).get_mut::<Slider>().unwrap().value = 7;
	world.update_local();
	render_html(&mut world, root).xpect_contains("value=\"7\"");
}

#[beet_core::test]
fn binding_entity_selector_text() {
	let mut world = world();
	register::<Slider>(&mut world);
	// the text binds the `bx:ref`-named entity's component via `@entity:name::`
	let root = spawn_bsx(
		&mut world,
		"<div><input bx:ref=\"slider\" {Slider{value:4}}/><span>{@entity:slider::Slider.value}</span></div>",
	);
	world.update_local();
	let children = world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.iter()
		.collect::<Vec<_>>();
	let input = children[0];
	let text = world.entity(children[1]).get::<Children>().unwrap()[0];
	read_i64(&world, text).xpect_eq(4);

	// reactive: the named component's edit reaches the text
	world.entity_mut(input).get_mut::<Slider>().unwrap().value = 6;
	world.update_local();
	read_i64(&world, text).xpect_eq(6);
}

#[beet_core::test]
fn binding_comp_spread_tuple() {
	let mut world = world();
	register::<Slider>(&mut world);
	// the tuple pairs the component insert with its binding, same entity
	let root = spawn_bsx(
		&mut world,
		"<entity {(Slider{value:3}, @comp:Slider.value)}/>",
	);
	world.update_local();
	world
		.entity(root)
		.get::<Slider>()
		.unwrap()
		.value
		.xpect_eq(3);
	read_i64(&world, root).xpect_eq(3);

	world.entity_mut(root).get_mut::<Slider>().unwrap().value = 8;
	world.update_local();
	read_i64(&world, root).xpect_eq(8);
}

// ---- reserved ref names -------------------------------------------------------

/// `BuildRoot` and `SnippetRoot` are the document's own root.
#[beet_core::test]
fn binding_comp_document_root() {
	for name in ["BuildRoot", "SnippetRoot"] {
		let mut world = world();
		register::<Slider>(&mut world);
		let root = spawn_bsx(
			&mut world,
			&format!(
				"<section {{Slider{{value:4}}}}><span>{{@entity:{name}::Slider.value}}</span></section>"
			),
		);
		world.update_local();
		let span = world.entity(root).get::<Children>().unwrap()[0];
		let text = world.entity(span).get::<Children>().unwrap()[0];
		read_i64(&world, text).xpect_eq(4);
		world.entity_mut(root).get_mut::<Slider>().unwrap().value = 6;
		world.update_local();
		read_i64(&world, text).xpect_eq(6);
	}
}

/// A stand-in carrying the reserved `Router` short name: lazy reserved
/// resolution is name-based, so the mechanism tests without `beet_router`
/// (whose real `Router`/`PageRoot` markers get their own crate's tests).
#[derive(Component, Default)]
struct Router;

#[beet_core::test]
fn binding_comp_router_lazy() {
	let mut world = world();
	register::<Slider>(&mut world);
	// build detached: no `Router` ancestor yet, the binding stays silent
	let input =
		spawn_bsx(&mut world, "<input value=@entity:Router::Slider.value/>");
	let router = world.spawn((Router, Slider { value: 5 })).id();
	world.update_local();
	world.with_state::<AttributeQuery, _>(|query| {
		query
			.find(input, "value")
			.unwrap()
			.1
			.clone()
			.xpect_eq(Value::Null);
	});

	// attaching beneath the router picks the binding up, even though the
	// bound component last changed before the marker was reachable
	world.entity_mut(input).insert(ChildOf(router));
	world.update_local();
	world.with_state::<AttributeQuery, _>(|query| {
		query
			.find(input, "value")
			.unwrap()
			.1
			.clone()
			.xpect_eq(Value::Int(5));
	});

	// reactive: a component edit on the router reaches the attribute
	world.entity_mut(router).get_mut::<Slider>().unwrap().value = 7;
	world.update_local();
	world.with_state::<AttributeQuery, _>(|query| {
		query
			.find(input, "value")
			.unwrap()
			.1
			.clone()
			.xpect_eq(Value::Int(7));
	});
}

#[beet_core::test]
fn component_tag_binding_doc() {
	let mut world = world();
	register::<Slider>(&mut world);
	let doc = world.spawn(Document::new(value!({ "level": 7i64 }))).id();
	let slider =
		spawn_bsx_under(&mut world, Some(doc), "<Slider value=@doc:level/>");
	world.update_local();
	world.update_local();
	world
		.entity(slider)
		.get::<Slider>()
		.unwrap()
		.value
		.xpect_eq(7);

	// write-back: a component edit reaches the document
	world.entity_mut(slider).get_mut::<Slider>().unwrap().value = 42;
	world.update_local();
	world.update_local();
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[FieldSegment::key("level")])
		.unwrap()
		.xpect_eq(42);
}

#[beet_core::test]
fn component_tag_binding_res() {
	let mut world = theme_world();
	register::<Slider>(&mut world);
	let slider = spawn_bsx(&mut world, "<Slider value=@res:Theme.contrast/>");
	world.update_local();
	world.update_local();
	world
		.entity(slider)
		.get::<Slider>()
		.unwrap()
		.value
		.xpect_eq(5);
	// the resource seeds outside-in, never clobbered by the component default
	world.resource::<Theme>().contrast.xpect_eq(5);

	world.resource_mut::<Theme>().contrast = 9;
	world.update_local();
	world.update_local();
	world
		.entity(slider)
		.get::<Slider>()
		.unwrap()
		.value
		.xpect_eq(9);
}

// ---- mdx --------------------------------------------------------------------

/// A template embedded in MDX markup, resolved by the markdown front-end
/// delegating to the BSX resolver.
#[template]
fn Greeting() -> impl Bundle {
	rsx! { <strong>"Hi"</strong> }
}

#[cfg(feature = "markdown_parser")]
#[beet_core::test]
fn mdx_resolves_embedded_template() {
	let mut app = world_ext::ui_app();
	app.install_types::<Greeting>();
	let world = app.world_mut();
	let bytes = MediaBytes::new_markdown("# Title\n\n<Greeting/>\n");
	let mut entity = world.spawn_empty();
	DocParsers::parse(ParseContext::new(&mut entity, &bytes)).unwrap();
	let root = entity.id();
	// the embedded <Greeting/> resolved to its <strong>Hi</strong> subtree.
	render_html(world, root).xpect_contains("<strong>Hi</strong>");
}

/// Parse markdown into a freshly spawned entity and render it to HTML.
#[cfg(feature = "markdown_parser")]
fn render_markdown(world: &mut World, md: &str) -> String {
	let bytes = MediaBytes::new_markdown(md);
	let mut entity = world.spawn_empty();
	DocParsers::parse(ParseContext::new(&mut entity, &bytes)).unwrap();
	let root = entity.id();
	render_html(world, root)
}

#[cfg(feature = "markdown_parser")]
#[beet_core::test]
fn mdx_plain_html_unchanged() {
	// a lowercase embedded element diffs as plain HTML, attributes intact.
	let mut world = world();
	render_markdown(&mut world, "<div class=\"card\">hi</div>")
		.xpect_contains("<div class=\"card\">")
		.xpect_contains("hi");
}

/// A component embedded in MDX as a wrapping tag (open + children + close
/// arriving as separate `pulldown-cmark` events) still resolves through BSX.
#[template]
fn Wrap() -> impl Bundle {
	rsx! { <section><slot/></section> }
}

#[cfg(feature = "markdown_parser")]
#[beet_core::test]
fn mdx_resolves_component_with_children() {
	let mut app = world_ext::ui_app();
	app.install_types::<Wrap>();
	let world = app.world_mut();
	// the open tag, the markdown body, and the close arrive as separate events;
	// the BSX build path resolves the whole subtree.
	render_markdown(world, "<Wrap>**bold**</Wrap>")
		.xpect_contains("<section>")
		.xpect_contains("<strong>bold</strong>");
}

// ---- html-mode subset -------------------------------------------------------

#[beet_core::test]
fn html_mode_is_plain_html() {
	let mut world = world();
	let bytes = MediaBytes::new_html("<div class=\"a\">hi</div>");
	let mut entity = world.spawn_empty();
	DocParsers::parse(ParseContext::new(&mut entity, &bytes)).unwrap();
	let div = entity.id();
	world
		.entity(div)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("div");
}

// ---- tag-position literals --------------------------------------------------

/// An enum component with a tuple variant, mirroring `beet_action`'s `Log`, so
/// `<LogLike::Message("hi")/>` builds the variant from the tag-position literal.
#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
enum LogLike {
	#[default]
	Name,
	Message(String),
}

/// A struct component, so `<SucceedTimesLike{max_times:2}/>` builds the field
/// from the tag-position literal.
#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct SucceedTimesLike {
	max_times: u32,
}

/// A bare marker component, used as a wrapping tag with children so the
/// component path (not the slot path) builds them as real entity children.
#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Holder;

#[beet_core::test]
fn tag_literal_name_coerces() {
	// a tag-position tuple literal builds the component from its fields, and the
	// string coerces into a real `Name` via `Name::new`.
	let mut world = world();
	register::<Name>(&mut world);
	let root = spawn_bsx(&mut world, "<Name(\"greeter\")/>");
	world
		.entity(root)
		.get::<Name>()
		.unwrap()
		.as_str()
		.xpect_eq("greeter");
}

#[beet_core::test]
fn tag_literal_enum_variant() {
	// a `::`-qualified tag-position literal builds the enum variant, exactly as the
	// `{Log::Message("..")}` spread does.
	let mut world = world();
	register::<LogLike>(&mut world);
	let root = spawn_bsx(&mut world, "<LogLike::Message(\"hi\")/>");
	world
		.entity(root)
		.get::<LogLike>()
		.cloned()
		.unwrap()
		.xpect_eq(LogLike::Message("hi".to_string()));
}

#[beet_core::test]
fn tag_literal_struct_sets_field() {
	// a tag-position struct literal builds the component with its field set.
	let mut world = world();
	register::<SucceedTimesLike>(&mut world);
	let root = spawn_bsx(&mut world, "<SucceedTimesLike{max_times:2}/>");
	world
		.entity(root)
		.get::<SucceedTimesLike>()
		.unwrap()
		.max_times
		.xpect_eq(2);
}

#[beet_core::test]
fn bare_component_tag_keeps_children() {
	// a bare uppercase component tag (no tag-position literal) builds its children
	// as real entity children, not slot content, with no unconsumed-slot error.
	let mut world = world();
	register::<Holder>(&mut world);
	let root = spawn_bsx(&mut world, "<Holder><span/><span/></Holder>");
	world.entity(root).contains::<Holder>().xpect_true();
	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(2);
	world
		.entity(children[0])
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("span");
}

#[beet_core::test]
fn tag_literal_component_with_children() {
	// a tag-position literal still builds children as entity children.
	let mut world = world();
	register::<Name>(&mut world);
	let root = spawn_bsx(&mut world, "<Name(\"parent\")><span/></Name>");
	world
		.entity(root)
		.get::<Name>()
		.unwrap()
		.as_str()
		.xpect_eq("parent");
	world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.len()
		.xpect_eq(1);
}

#[beet_core::test]
fn tag_literal_resolves_entity_ref() {
	// a tag-position literal resolves a `$name` into an `Entity` field, the
	// `<GoTo($target)/>` state-machine shape.
	let mut world = world();
	register::<Bound>(&mut world);
	let root = spawn_bsx(
		&mut world,
		"<div><Bound($target)/><b bx:ref=\"target\"/></div>",
	);
	let children = world
		.entity(root)
		.get::<Children>()
		.unwrap()
		.iter()
		.collect::<Vec<_>>();
	let target = children[1];
	world
		.entity(children[0])
		.get::<Bound>()
		.unwrap()
		.0
		.xpect_eq(target);
}

/// A component with a `#[reflect(@RequiredField)]` field, so the BSX loader errors
/// when it is authored without it.
#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct NeedsField {
	#[reflect(@RequiredField)]
	required: u32,
	optional: u32,
}

#[beet_core::test]
fn required_field_missing_errors() {
	// a missing `#[reflect(@RequiredField)]` field is a graceful build error,
	// naming the type and field.
	let mut world = world();
	register::<NeedsField>(&mut world);
	build_error(&mut world, "<NeedsField{optional:1}/>")
		.xpect_contains("missing required field 'required'");
}

#[beet_core::test]
fn required_field_present_succeeds() {
	// the same component authored with its required field builds fine.
	let mut world = world();
	register::<NeedsField>(&mut world);
	let root = spawn_bsx(&mut world, "<NeedsField{required:7}/>");
	world
		.entity(root)
		.get::<NeedsField>()
		.unwrap()
		.required
		.xpect_eq(7);
}
