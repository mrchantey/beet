//! Tests for the BSX parser and its resolution into the document-wired template
//! tree (Task 4 gate): literals with type-inferred reflect, `#`/`$` references,
//! spreads, `bx:` directives, slots, and events, asserted on the built tree.
//! A `.bsx` snippet must resolve to the same tree the equivalent `rsx!` lowers.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;
use bevy::reflect::GetTypeRegistration;

/// A spawn-capable template world.
fn world() -> World { ui_world() }

/// Register a reflect type into the world's [`AppTypeRegistry`].
fn register<T: GetTypeRegistration>(world: &mut World) {
	world.resource_mut::<AppTypeRegistry>().write().register::<T>();
}

/// Parse a `.bsx` source into a container and return its first content child
/// (the parser builds parsed content as children of the parse target).
fn spawn_bsx(world: &mut World, source: &str) -> Entity {
	let container = parse_bsx(world, None, source);
	world.entity(container).get::<Children>().unwrap()[0]
}

/// Parse a `.bsx` source into a container under `parent` (so the document
/// machinery links before the tree is built), returning the content child.
fn spawn_bsx_under(
	world: &mut World,
	parent: Option<Entity>,
	source: &str,
) -> Entity {
	let container = parse_bsx(world, parent, source);
	world.entity(container).get::<Children>().unwrap()[0]
}

/// Parse a `.bsx` source into a freshly spawned container entity, returning the
/// container (whose children are the parsed roots).
fn parse_bsx(world: &mut World, parent: Option<Entity>, source: &str) -> Entity {
	let bytes = MediaBytes::new_bsx(source);
	let mut entity = match parent {
		Some(parent) => world.spawn(ChildOf(parent)),
		None => world.spawn_empty(),
	};
	BsxParser::bsx()
		.parse(ParseContext::new(&mut entity, &bytes))
		.unwrap();
	entity.id()
}

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

// ---- basic markup -----------------------------------------------------------

#[beet_core::test]
fn element_and_text() {
	let mut world = world();
	let root = spawn_bsx(&mut world, "<div>hello</div>");
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("div");
	descendant_values(&world, root).xpect_eq(vec!["hello".to_string()]);
}

#[beet_core::test]
fn nested_elements() {
	let mut world = world();
	let root = spawn_bsx(&mut world, "<div><span>inner</span></div>");
	let span = world.entity(root).get::<Children>().unwrap()[0];
	world.entity(span).get::<Element>().unwrap().tag().xpect_eq("span");
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
		query.find(root, "id").unwrap().1.as_str().unwrap().xpect_eq("main");
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
		.xpect_eq(Sized { width: 8.0, height: 4.0 });
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
		.xpect_eq(Aligned { align: Align::Center });
}

// ---- #references ------------------------------------------------------------

#[beet_core::test]
fn field_ref_binds_document() {
	let mut world = world();
	let doc = world.spawn(Document::new(val!({ "count": 7 }))).id();
	let root = spawn_bsx_under(&mut world, Some(doc), "<span>{#count}</span>");
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
	let doc = world.spawn(Document::new(val!({}))).id();
	let _root = spawn_bsx_under(&mut world, Some(doc), "<span>{#count=5}</span>");
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
	fn default() -> Self { Self { to: Entity::PLACEHOLDER } }
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
	world.entity(root).get::<Sized>().unwrap().width.xpect_eq(2.0);
}

// ---- bx:scope ---------------------------------------------------------------

#[beet_core::test]
fn scope_prefixes_descendant_fields() {
	let mut world = world();
	let doc = world
		.spawn(Document::new(val!({ "user": { "name": "Alice" } })))
		.id();
	let root = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<div bx:scope=\"user\"><span>{#name}</span></div>",
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
		.spawn(Document::new(val!({ "items": ["a", "b", "c"] })))
		.id();
	let list = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<ul bx:for=\"items\" bx:key=\"i\"><li>x</li></ul>",
	);
	world.update_local();
	world.update_local();
	// one <li> child per item.
	world
		.entity(list)
		.get::<Children>()
		.map(|children| children.len())
		.unwrap()
		.xpect_eq(3);
}

// ---- slots ------------------------------------------------------------------

#[beet_core::test]
fn slots_via_registered_template() {
	let mut world = world();
	// register a `.bsx` template with a named and a default slot.
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source(
			"Card",
			"<section><Slot name=\"header\">Fallback</Slot><Slot/></section>",
		)
		.unwrap();
	world.insert_resource(registry);

	let root = spawn_bsx(
		&mut world,
		"<Card><div>Body</div><h1 bx:slot=\"header\">Title</h1></Card>",
	);
	let html = render_html(&mut world, root);
	// the header slot took the caller's title, the default slot the body.
	html.clone().xpect_contains("Title");
	html.clone().xpect_contains("Body");
	html.xpect_contains("<section>");
}

#[beet_core::test]
fn slot_fallback_when_no_content() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section><Slot name=\"header\">Fallback</Slot></section>")
		.unwrap();
	world.insert_resource(registry);
	let root = spawn_bsx(&mut world, "<Card/>");
	render_html(&mut world, root).xpect_contains("Fallback");
}

// ---- events -----------------------------------------------------------------

#[beet_core::test]
fn click_increments_field() {
	let mut world = world();
	let doc = world.spawn(Document::new(val!({ "count": 0 }))).id();
	let button = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<button bx:click=\"increment#count\">+</button>",
	);
	world.update_local();
	// fire a pointer-down on the button: the verb runs against its bound Value.
	let pointer = world.spawn_empty().id();
	world
		.entity_mut(button)
		.trigger(move |target| PointerDown { target, pointer });
	world.flush();
	world
		.entity(button)
		.get::<Value>()
		.unwrap()
		.as_i64()
		.unwrap()
		.xpect_eq(1);
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
	let mut world = world();
	world.register_template::<Greeting>();
	let bytes = MediaBytes::new_markdown("# Title\n\n<Greeting/>\n");
	let mut entity = world.spawn_empty();
	MarkdownParser::new()
		.parse(ParseContext::new(&mut entity, &bytes))
		.unwrap();
	let root = entity.id();
	// the embedded <Greeting/> resolved to its <strong>Hi</strong> subtree.
	render_html(&mut world, root).xpect_contains("<strong>Hi</strong>");
}

/// Parse markdown into a freshly spawned entity and render it to HTML.
#[cfg(feature = "markdown_parser")]
fn render_markdown(world: &mut World, md: &str) -> String {
	let bytes = MediaBytes::new_markdown(md);
	let mut entity = world.spawn_empty();
	MarkdownParser::new()
		.parse(ParseContext::new(&mut entity, &bytes))
		.unwrap();
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
	let mut world = world();
	world.register_template::<Wrap>();
	// the open tag, the markdown body, and the close arrive as separate events;
	// the BSX build path resolves the whole subtree.
	render_markdown(&mut world, "<Wrap>**bold**</Wrap>")
		.xpect_contains("<section>")
		.xpect_contains("<strong>bold</strong>");
}

// ---- html-mode subset -------------------------------------------------------

#[beet_core::test]
fn html_mode_is_plain_html() {
	let mut world = world();
	let bytes = MediaBytes::new_html("<div class=\"a\">hi</div>");
	let container = parse_bsx_html(&mut world, &bytes);
	// the parsed content is a child of the container parse target.
	let div = world.entity(container).get::<Children>().unwrap()[0];
	world.entity(div).get::<Element>().unwrap().tag().xpect_eq("div");
}

/// Parse HTML-mode bytes into a container, returning the container entity.
fn parse_bsx_html(world: &mut World, bytes: &MediaBytes) -> Entity {
	let mut entity = world.spawn_empty();
	BsxParser::html()
		.parse(ParseContext::new(&mut entity, bytes))
		.unwrap();
	entity.id()
}
