//! Tests for a template's prop schema: registered from its typed signature,
//! held to as a document lowers, with the attributes beyond its props
//! forwarded.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A spawn-capable template world.
fn world() -> World { world_ext::ui_world() }

/// Parse a `.bsx` source into a freshly spawned root.
fn parse_bsx(
	world: &mut World,
	source: &str,
) -> core::result::Result<Entity, ParseError> {
	let bytes = MediaBytes::new_bsx(source);
	let mut entity = world.spawn_empty();
	DocParsers::parse(ParseContext::new(&mut entity, &bytes))?;
	entity.id().xok()
}

// ---- schema typing: a Rust template registers its prop schema ----------------

/// A template with an optional and a required prop, authoring its schema via its
/// typed signature.
#[template]
fn Badge(#[prop(required)] label: String, count: u32) -> impl Bundle {
	let _ = (label, count);
	rsx! { <span/> }
}

#[beet_core::test]
fn rust_template_registers_schema() {
	let mut app = world_ext::ui_app();
	app.install_types::<Badge>();
	let world = app.world_mut();
	// the prop schema is attached alongside the build bridge, derived from the
	// typed signature: `label` required, `count` optional.
	let registry = world.resource::<AppTypeRegistry>().clone();
	let schema = ValueSchema::template_by_name(&registry, "Badge").unwrap();
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

// ---- missing required prop -> an error of the document ------------------------

/// A required prop left out, or a prop of the wrong type, refuses the document
/// as it lowers, naming the tag.
#[beet_core::test]
fn missing_required_is_refused() {
	let mut app = world_ext::ui_app();
	app.install_types::<Badge>();
	for source in ["<Badge count=1/>", "<Badge label=\"hi\" count=\"lots\"/>"] {
		parse_bsx(app.world_mut(), source)
			.unwrap_err()
			.to_string()
			.xpect_contains("Badge");
	}
}

#[beet_core::test]
fn macro_required_prop_present_ok() {
	let mut app = world_ext::ui_app();
	app.install_types::<Badge>();
	let world = app.world_mut();
	parse_bsx(world, "<Badge label=\"hi\" count=1/>").unwrap();
}

// ---- attributes beyond the props forward to the root element -----------------

/// Render `root` to HTML.
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

/// A Rust template forwards the attributes beyond its props onto the element
/// it builds: `hidden` and a `class` land on `<Badge>`'s `<span>`, its props
/// do not.
#[beet_core::test]
fn rust_template_forwards_attributes() {
	let mut app = world_ext::ui_app();
	app.install_types::<Badge>();
	let world = app.world_mut();
	let root = parse_bsx(world, r#"<Badge label="hi" class="extra" hidden/>"#)
		.unwrap();
	render_html(world, root)
		.xpect_contains(" hidden")
		.xpect_contains(r#"class="extra""#)
		.xnot()
		.xpect_contains("label=");
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
	let doc = world.spawn(Document::new(value!({ "level": 9i64 }))).id();
	// `<Slider value=@doc:level>` binds document `level` to `Slider.value`, both ways.
	let bytes = MediaBytes::new_bsx("<Slider value=@doc:level/>");
	let root = {
		let mut entity = world.spawn(ChildOf(doc));
		DocParsers::parse(ParseContext::new(&mut entity, &bytes)).unwrap();
		entity.id()
	};
	world.update_local();
	world.update_local();

	// the document value reached the component field.
	let slider = root;
	world
		.entity(slider)
		.get::<Slider>()
		.unwrap()
		.value
		.xpect_eq(9);
	// the binding components are present on the component entity.
	world.entity(slider).contains::<FieldRef>().xpect_true();
	world
		.entity(slider)
		.contains::<ReflectFieldRef>()
		.xpect_true();
}
