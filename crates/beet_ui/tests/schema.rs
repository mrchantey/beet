//! Tests for Task 5: schemas, the registry, and references.
//!
//! Covers the gate: schema seed/assert, a missing required prop surfacing a
//! graceful `TemplateError` (macro and loader paths), file-path module
//! resolution (`<path::to::X>` from `path/to/X.bsx`), a composable schema
//! spanning two templates, `LoadTemplate` draining on async schema resolution,
//! and the remote-template stub wired into the pending set.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;

/// A spawn-capable template world.
fn world() -> World { ui_world() }

/// Parse a `.bsx` source into a freshly spawned container, returning the root.
fn parse_bsx(world: &mut World, source: &str) -> Entity {
	let bytes = MediaBytes::new_bsx(source);
	let mut entity = world.spawn_empty();
	BsxParser::bsx()
		.parse(ParseContext::new(&mut entity, &bytes))
		.unwrap();
	entity.id()
}

/// Whether any entity beneath `root` (inclusive) carries a [`TemplateError`].
fn has_template_error(world: &World, root: Entity) -> bool {
	let mut stack = vec![root];
	while let Some(entity) = stack.pop() {
		if world.entity(entity).contains::<TemplateError>() {
			return true;
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			stack.extend(children.iter());
		}
	}
	false
}

// ---- schema typing: a Rust template registers its prop schema ----------------

/// A template with an optional and a required prop, authoring its schema via its
/// typed signature.
#[template]
fn Badge(
	#[prop(required)] label: String,
	count: u32,
) -> impl Bundle {
	let _ = (label, count);
	rsx! { <span/> }
}

#[beet_core::test]
fn rust_template_registers_schema() {
	let mut world = world();
	world.register_template::<Badge>();
	// the prop schema is attached alongside the build bridge, derived from the
	// typed signature: `label` required, `count` optional.
	let registry = world.resource::<AppTypeRegistry>().clone();
	let schema = template_schema_by_name(&registry, "Badge").unwrap();
	let ValueSchema::Struct(struct_schema) = schema else {
		panic!("expected a struct schema");
	};
	let label = struct_schema
		.fields
		.iter()
		.find(|field| field.key == "label")
		.unwrap();
	label.required.xpect_true();
	let count = struct_schema
		.fields
		.iter()
		.find(|field| field.key == "count")
		.unwrap();
	count.required.xpect_false();
}

// ---- missing required prop -> graceful TemplateError (macro path) ------------

#[beet_core::test]
fn macro_missing_required_rides_template_error() {
	let mut world = world();
	world.register_template::<Badge>();
	// `<Badge/>` without `label` builds with the required prop unset.
	let root = parse_bsx(&mut world, "<Badge count=1/>");
	// never panicked; the missing required prop rode a TemplateError.
	has_template_error(&world, root).xpect_true();
}

#[beet_core::test]
fn macro_required_prop_present_ok() {
	let mut world = world();
	world.register_template::<Badge>();
	let root = parse_bsx(&mut world, "<Badge label=\"hi\" count=1/>");
	has_template_error(&world, root).xpect_false();
}

// ---- missing required prop -> graceful TemplateError (loader path) -----------

#[beet_core::test]
fn loader_missing_required_rides_template_error() {
	let mut world = world();
	// a BSX template declaring a required `title` prop via a `bx:schema` block.
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source(
			"Card",
			"<script type=\"json\" bx:schema>{ \"title\": { \"type\": \"string\", \"required\": true } }</script><section><h1>{@doc:title}</h1></section>",
		)
		.unwrap();
	world.insert_resource(registry);

	// `<Card/>` without `title` violates the schema -> graceful error.
	let root = parse_bsx(&mut world, "<Card/>");
	has_template_error(&world, root).xpect_true();

	// providing the required prop validates.
	let ok = parse_bsx(&mut world, "<Card title=\"hello\"/>");
	has_template_error(&world, ok).xpect_false();
}

// ---- module-path resolution: <path::to::X> from path/to/X.bsx ----------------

#[beet_core::test]
fn module_path_resolution_from_directory() {
	let mut world = world();
	// lay out a template directory: <dir>/path/to/X.bsx (a process-unique dir so
	// parallel runs do not collide).
	let dir = std::env::temp_dir().join(format!(
		"beet_bsx_templates_{}",
		std::process::id()
	));
	let _ = fs_ext::remove(&dir);
	let file = dir.join("path/to/X.bsx");
	fs_ext::write(&file, "<strong>indexed</strong>").unwrap();

	// the registration pass indexes the directory, so `<path::to::X>` resolves.
	world.register_bsx_templates(&dir).unwrap();
	world
		.resource::<BsxTemplateRegistry>()
		.contains("path::to::X")
		.xpect_true();

	let root = parse_bsx(&mut world, "<path::to::X/>");
	let html = HtmlRenderer::new()
		.render(&mut RenderContext::new(root, &mut world))
		.unwrap()
		.to_string();
	let _ = fs_ext::remove(&dir);
	html.xpect_contains("indexed");
}

// ---- composable schema spanning two templates --------------------------------

#[beet_core::test]
fn composable_schema_validates_recursively() {
	let mut world = world();
	// register two BSX templates: `TodoItem` (a struct) and `TodoList` whose
	// `items` is a list of `TodoItem` (a composable reference).
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source(
			"TodoItem",
			"<script type=\"json\" bx:schema>{ \"label\": { \"type\": \"string\", \"required\": true } }</script><li>{@doc:label}</li>",
		)
		.unwrap();
	registry
		.insert_source(
			"TodoList",
			"<script type=\"json\" bx:schema>{ \"items\": { \"items\": \"TodoItem\", \"required\": true } }</script><ul>x</ul>",
		)
		.unwrap();
	world.insert_resource(registry);
	// mirror the BSX schemas into the shared schema registry so the `TodoItem`
	// reference resolves.
	world.register_bsx_schemas();

	// a valid list of todo items passes recursive validation.
	let ok = parse_bsx(
		&mut world,
		"<TodoList items={[{label:\"buy milk\"}]}/>",
	);
	has_template_error(&world, ok).xpect_false();

	// a todo item missing its required `label` fails recursively.
	let bad = parse_bsx(&mut world, "<TodoList items={[{}]}/>");
	has_template_error(&world, bad).xpect_true();
}

// ---- reflect-field binding: <MyComponent value=@doc:path> --------------------

#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Component, Default)]
struct Slider {
	value: i64,
}

#[beet_core::test]
fn component_field_binds_document() {
	let mut world = world();
	world
		.resource_mut::<AppTypeRegistry>()
		.write()
		.register::<Slider>();
	let doc = world.spawn(Document::new(val!({ "level": 9i64 }))).id();
	// `<Slider value=@doc:level>` binds document `level` to `Slider.value`, both ways.
	let bytes = MediaBytes::new_bsx("<Slider value=@doc:level/>");
	let root = {
		let mut entity = world.spawn(ChildOf(doc));
		BsxParser::bsx()
			.parse(ParseContext::new(&mut entity, &bytes))
			.unwrap();
		entity.id()
	};
	world.update_local();
	world.update_local();

	// the document value reached the component field.
	let slider = world.entity(root).get::<Children>().unwrap()[0];
	world.entity(slider).get::<Slider>().unwrap().value.xpect_eq(9);
	// the binding components are present on the component entity.
	world.entity(slider).contains::<FieldRef>().xpect_true();
	world.entity(slider).contains::<ReflectFieldRef>().xpect_true();
}

// ---- async schema resolution drains LoadTemplate -----------------------------

#[beet_core::test(timeout_ms = 5000)]
async fn async_remote_schema_defers_load() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins.set(TaskPoolPlugin {
		task_pool_options: TaskPoolOptions::with_num_threads(2),
	}));
	app.add_plugins((AsyncPlugin, TemplatePlugin, DocumentPlugin));

	// a BSX template whose schema is fetched remotely (stubbed), deferring load.
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source(
			"Remote",
			"<script bx:schema src=\"https://example.com/schema.json\"></script><div>remote</div>",
		)
		.unwrap();
	app.world_mut().insert_resource(registry);

	let load_state = Store::new(None);
	let ls = load_state.clone();
	app.world_mut()
		.add_observer(move |ev: On<LoadTemplate>| ls.set(Some(ev.is_error)));

	// spawn the template; the remote schema parks a pending dependency.
	let bytes = MediaBytes::new_bsx("<Remote/>");
	let root = {
		let world = app.world_mut();
		let mut entity = world.spawn_empty();
		BsxParser::bsx()
			.parse(ParseContext::new(&mut entity, &bytes))
			.unwrap();
		entity.id()
	};

	// LoadTemplate has not fired: the remote schema is still pending.
	load_state.get().xpect_none();
	let pending = app.world().entity(root).get::<TemplatePending>();
	pending.map(|pending| pending.is_empty()).xpect_eq(Some(false));

	// pump frames until the async fetch resolves and the pending set drains.
	for _ in 0..200 {
		app.update();
		if load_state.get().is_some() {
			break;
		}
		time_ext::sleep_millis(5).await;
	}
	// LoadTemplate fired once the remote schema resolved.
	load_state.get().xpect_eq(Some(false));
}

// ---- remote-template stub wired into the pending set -------------------------

#[beet_core::test(timeout_ms = 5000)]
async fn remote_template_stub_defers_load() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins.set(TaskPoolPlugin {
		task_pool_options: TaskPoolOptions::with_num_threads(2),
	}));
	app.add_plugins((AsyncPlugin, TemplatePlugin, DocumentPlugin));

	let load_state = Store::new(None);
	let ls = load_state.clone();
	app.world_mut()
		.add_observer(move |ev: On<LoadTemplate>| ls.set(Some(ev.is_error)));

	// `<Template src="..">` registers a pending fetch into the root's pending set.
	let bytes = MediaBytes::new_bsx("<Template src=\"https://example.com/Todo.bsx\"/>");
	let root = {
		let world = app.world_mut();
		let mut entity = world.spawn_empty();
		BsxParser::bsx()
			.parse(ParseContext::new(&mut entity, &bytes))
			.unwrap();
		entity.id()
	};

	// the fetch is pending, so LoadTemplate is deferred.
	load_state.get().xpect_none();
	app.world()
		.entity(root)
		.get::<TemplatePending>()
		.map(|pending| pending.is_empty())
		.xpect_eq(Some(false));

	// pump frames until the stubbed fetch resolves and the pending set drains.
	for _ in 0..200 {
		app.update();
		if load_state.get().is_some() {
			break;
		}
		time_ext::sleep_millis(5).await;
	}
	load_state.get().xpect_eq(Some(false));
}
