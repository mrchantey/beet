//! Tests for the BSX parser and its resolution into the document-wired template
//! tree (Task 4 gate): literals with type-inferred reflect, `@` bindings,
//! `#`/`$` references, spreads, `bx:` directives, slots, and events, asserted
//! on the built tree.
//! A `.bsx` snippet must resolve to the same tree the equivalent `rsx!` lowers.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;
use bevy::reflect::GetTypeRegistration;

/// A spawn-capable template world.
fn world() -> World { ui_world() }

/// Register a reflect type into the world's [`AppTypeRegistry`].
fn register<T: GetTypeRegistration>(world: &mut World) {
	world
		.resource_mut::<AppTypeRegistry>()
		.write()
		.register::<T>();
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
fn parse_bsx(
	world: &mut World,
	parent: Option<Entity>,
	source: &str,
) -> Entity {
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
	let doc = world.spawn(Document::new(val!({ "count": 7 }))).id();
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
	let doc = world.spawn(Document::new(val!({}))).id();
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
fn spread_on_bsx_template_tag() {
	let mut world = world();
	register::<Marker>(&mut world);
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section><Slot/></section>")
		.unwrap();
	world.insert_resource(registry);
	let root = spawn_bsx(&mut world, "<Card {Marker}>Body</Card>");
	world.entity(root).contains::<Marker>().xpect_true();
	render_html(&mut world, root).xpect_contains("Body");
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

// ---- props store ------------------------------------------------------------

/// Spawn a body-position props binding (what `@prop:<key>` will lower to once
/// the syntax lands) under `parent`, returning the value-bearing entity.
fn spawn_props_probe(world: &mut World, parent: Entity, key: &str) -> Entity {
	world
		.spawn((
			ChildOf(parent),
			Value::default(),
			FieldRef::new(key).with_document(DocumentPath::Props),
		))
		.id()
}

/// Read the local [`Value`] of `entity`.
fn read_value(world: &World, entity: Entity) -> Value {
	world.entity(entity).get::<Value>().unwrap().clone()
}

#[beet_core::test]
fn template_props_store_literal() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section><Slot/></section>")
		.unwrap();
	world.insert_resource(registry);

	let card = spawn_bsx(&mut world, "<Card title=\"hi\"/>");
	// the literal prop materialized into a props store on the template entity.
	world.entity(card).contains::<PropsDocument>().xpect_true();
	world
		.entity(card)
		.get::<Document>()
		.unwrap()
		.get_field_ref(&[FieldSegment::key("title")])
		.unwrap()
		.clone()
		.xpect_eq(Value::Str("hi".into()));

	// a body binding reads the prop through `DocumentPath::Props`.
	let probe = spawn_props_probe(&mut world, card, "title");
	world.update_local();
	read_value(&world, probe).xpect_eq(Value::Str("hi".into()));
}

#[beet_core::test]
fn template_props_bound_reactively() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section><Slot/></section>")
		.unwrap();
	world.insert_resource(registry);

	let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
	let card =
		spawn_bsx_under(&mut world, Some(doc), "<Card title=@doc:name/>");
	world.update_local();
	world.update_local();

	// the binding entity is related via `AttributeOf`, never rendered.
	render_html(&mut world, card).xnot().xpect_contains("Alice");
	// the bound value reached the props store.
	world
		.entity(card)
		.get::<Document>()
		.unwrap()
		.get_field_ref(&[FieldSegment::key("title")])
		.unwrap()
		.clone()
		.xpect_eq(Value::Str("Alice".into()));

	// a body binding reads the caller-bound prop, reactively.
	let probe = spawn_props_probe(&mut world, card, "title");
	world.update_local();
	read_value(&world, probe).xpect_eq(Value::Str("Alice".into()));

	world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
		val!({ "name": "Bob" });
	world.update_local();
	world.update_local();
	read_value(&world, probe).xpect_eq(Value::Str("Bob".into()));
}

#[beet_core::test]
fn nested_template_props_do_not_leak() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry.insert_source("Inner", "<span></span>").unwrap();
	registry
		.insert_source("Outer", "<div><Inner title=\"inner_val\"/></div>")
		.unwrap();
	world.insert_resource(registry);

	let outer = spawn_bsx(&mut world, "<Outer title=\"outer_val\"/>");
	// the inner template entity carries its own store.
	let inner = world
		.entity(outer)
		.get::<Children>()
		.unwrap()
		.iter()
		.find(|child| world.entity(*child).contains::<PropsDocument>())
		.unwrap();

	// each body binding resolves its nearest store, no leakage either way.
	let inner_probe = spawn_props_probe(&mut world, inner, "title");
	let outer_probe = spawn_props_probe(&mut world, outer, "title");
	world.update_local();
	read_value(&world, inner_probe).xpect_eq(Value::Str("inner_val".into()));
	read_value(&world, outer_probe).xpect_eq(Value::Str("outer_val".into()));
}

#[beet_core::test]
fn ancestor_ref_in_template_body_skips_props_store() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section>{@doc:name}</section>")
		.unwrap();
	world.insert_resource(registry);

	let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
	// the tag's own store also carries a `name` prop, which must not shadow
	// the user document for the body's `@doc:name` ancestor binding.
	let card =
		spawn_bsx_under(&mut world, Some(doc), "<Card name=\"shadow\"/>");
	world.update_local();

	let text = world.entity(card).get::<Children>().unwrap()[0];
	read_value(&world, text).xpect_eq(Value::Str("Alice".into()));
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
		.insert_source(
			"Card",
			"<section><Slot name=\"header\">Fallback</Slot></section>",
		)
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
		"<button bx:click=increment{ field: @doc:count }>+</button>",
	);
	world.update_local();
	// the host carries no mirror: the verb writes the document, not a local Value.
	world.entity(button).contains::<FieldRef>().xpect_false();
	world.entity(button).contains::<Value>().xpect_false();
	// fire a pointer-down on the button: the verb writes the ancestor document.
	let pointer = world.spawn_empty().id();
	world
		.entity_mut(button)
		.trigger(move |target| PointerDown { target, pointer });
	world.flush();
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[FieldSegment::key("count")])
		.unwrap()
		.xpect_eq(1);
}

#[beet_core::test]
fn click_increments_by_amount() {
	let mut world = world();
	let doc = world.spawn(Document::new(val!({ "count": 0 }))).id();
	let button = spawn_bsx_under(
		&mut world,
		Some(doc),
		"<button bx:click=increment{ field: @doc:count, amount: 5 }>+</button>",
	);
	world.update_local();
	let pointer = world.spawn_empty().id();
	world
		.entity_mut(button)
		.trigger(move |target| PointerDown { target, pointer });
	world.flush();
	world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[FieldSegment::key("count")])
		.unwrap()
		.xpect_eq(5);
}

/// The counter page shape: a `bx:scope`, an initializing text binding, and an
/// event verb mutating the same scoped field, with no pre-existing document.
/// Regressions covered: the scoped init path corrupting the document
/// (`{counter: 0}` instead of `{counter: {count: 0}}`), and the event host
/// carrying no mirror that could leak into the button's rendered label. The
/// click drives the end-to-end verb -> scoped-document -> display-binding path.
#[beet_core::test]
fn scoped_counter_page() {
	let mut world = world();
	let root = spawn_bsx(
		&mut world,
		r#"<article bx:scope="counter"><p>clicked {@doc:count=0} times</p><button bx:click=increment{ field: @doc:count }>More</button></article>"#,
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

	// click the button: the verb resolves the scoped field and writes the document
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
	let pointer = world.spawn_empty().id();
	world
		.entity_mut(button)
		.trigger(move |target| PointerDown { target, pointer });
	world.flush();
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

// ---- live TUI counter (the `bsx_site` reactivity example) ---------------------

/// The `examples/bsx_site/routes/counter.bsx` markup, the no-code reactivity
/// example: a scoped document, a display binding, and the `increment`/`decrement`
/// verbs (one with an explicit `amount` arg) mutating it.
#[cfg(feature = "tui")]
const COUNTER_BSX: &str = r#"<article bx:scope="counter">
	<widgets::Card title="Counter">
		<p>You have clicked {@doc:count=0} times.</p>
		<button bx:click=increment{ field: @doc:count, amount: 1 }>More</button>
		<button bx:click=decrement{ field: @doc:count }>Less</button>
	</widgets::Card>
</article>"#;

/// Drive `counter.bsx` through the real live-charcell stack (the terminal target):
/// a click runs the verb, document-sync fans the change to the display binding,
/// and the charcell renderer repaints. Asserts the *rendered frame text* changes,
/// not just the document, exercising the click -> repaint path end to end.
#[cfg(feature = "tui")]
#[beet_core::test]
fn counter_bsx_repaints_in_live_tui() {
	use bevy::math::UVec2;

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, CharcellTuiPlugin));
	// the site-local `<widgets::Card>` template the counter composes
	app.world_mut()
		.resource_mut::<BsxTemplateRegistry>()
		.insert_source(
			"widgets::Card",
			"<section><h2>{@prop:title}</h2><Slot/></section>",
		)
		.unwrap();
	// the host buffer the charcell pipeline paints into, plus a channel terminal
	// so the run is headless and deterministic.
	let (channel, terminal) = ChannelTerminal::new(TerminalConfig::default());
	let host = app
		.world_mut()
		.spawn((channel, terminal, DoubleBuffer::new(UVec2::new(48, 12))))
		.id();
	app.update();
	// parse the counter markup as the host's content tree.
	let bytes = MediaBytes::new_bsx(COUNTER_BSX);
	BsxParser::bsx()
		.parse(ParseContext::new(
			&mut app.world_mut().entity_mut(host),
			&bytes,
		))
		.unwrap();

	// step until the scoped `@doc:count=0` init reaches the rendered frame.
	let frame = step_until(&mut app, host, "clicked 0 times");
	frame.xpect_contains("Counter");

	// click "More": fire a `PointerDown` on the increment button, like the hit-test
	// does for a real cursor press.
	let more = find_button(&mut app, host, "More");
	click(&mut app, more);
	// the verb wrote the document; the next frames sync the binding and repaint.
	step_until(&mut app, host, "clicked 1 times");

	// click "Less": the decrement verb walks back to the same scoped field.
	let less = find_button(&mut app, host, "Less");
	click(&mut app, less);
	step_until(&mut app, host, "clicked 0 times");
}

/// Trigger a `PointerDown` on `entity`, then flush so the queued verb command runs.
#[cfg(feature = "tui")]
fn click(app: &mut App, entity: Entity) {
	let pointer = app.world_mut().spawn_empty().id();
	app.world_mut()
		.entity_mut(entity)
		.trigger(move |target| PointerDown { target, pointer });
	app.world_mut().flush();
}

/// The `<button>` entity beneath `host` whose rendered text is `label`.
#[cfg(feature = "tui")]
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
#[cfg(feature = "tui")]
fn step_until(app: &mut App, host: Entity, needle: &str) -> String {
	for _ in 0..50 {
		app.update();
		let frame = app
			.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.front_buffer()
			.render_plain();
		if frame.contains(needle) {
			return frame;
		}
	}
	let frame = app
		.world()
		.get::<DoubleBuffer>(host)
		.unwrap()
		.front_buffer()
		.render_plain();
	panic!("frame never contained {needle:?}:\n{frame}");
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
	let doc = world.spawn(Document::new(val!({ "count": 7 }))).id();
	let root =
		spawn_bsx_under(&mut world, Some(doc), "<span>{@doc:count}</span>");
	world.update_local();
	let text = world.entity(root).get::<Children>().unwrap()[0];
	world.entity(text).contains::<FieldRef>().xpect_true();
	read_i64(&world, text).xpect_eq(7);

	// reactive: a document change reaches the text
	world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
		val!({ "count": 8 });
	world.update_local();
	read_i64(&world, text).xpect_eq(8);
}

#[beet_core::test]
fn binding_doc_init_seeds_when_missing() {
	let mut world = world();
	let doc = world.spawn(Document::new(val!({}))).id();
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
	let doc = world.spawn(Document::new(val!({ "name": "Ada" }))).id();
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

/// Build `source` and return the template error.
fn build_error(world: &mut World, source: &str) -> String {
	let nodes = parse_document(source, &BsxParseConfig::bsx()).unwrap();
	world
		.spawn_template(BsxTemplate::container(
			nodes,
			BsxTemplateRegistry::default(),
		))
		.map(|entity| entity.id())
		.unwrap_err()
		.to_string()
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

#[beet_core::test]
fn binding_comp_snippet_root() {
	let mut world = world();
	register::<Slider>(&mut world);
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source(
			"Probe",
			"<section><span>{@entity:SnippetRoot::Slider.value}</span></section>",
		)
		.unwrap();
	world.insert_resource(registry);
	// the tag-site spread lands on the template entity, the body's snippet root
	let probe = spawn_bsx(&mut world, "<Probe {Slider{value:4}}/>");
	world.update_local();
	let span = world.entity(probe).get::<Children>().unwrap()[0];
	let text = world.entity(span).get::<Children>().unwrap()[0];
	read_i64(&world, text).xpect_eq(4);

	// reactive: the snippet root's component edit reaches the text
	world.entity_mut(probe).get_mut::<Slider>().unwrap().value = 6;
	world.update_local();
	read_i64(&world, text).xpect_eq(6);
}

#[beet_core::test]
fn binding_comp_build_root() {
	let mut world = world();
	register::<Slider>(&mut world);
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source(
			"Probe",
			"<span>{@entity:BuildRoot::Slider.value}</span>",
		)
		.unwrap();
	world.insert_resource(registry);
	// the build root is the parse container, above the template's snippet root
	let container = parse_bsx(&mut world, None, "<div><Probe/></div>");
	world.entity_mut(container).insert(Slider { value: 3 });
	world.update_local();
	let div = world.entity(container).get::<Children>().unwrap()[0];
	let probe = world.entity(div).get::<Children>().unwrap()[0];
	let text = world.entity(probe).get::<Children>().unwrap()[0];
	read_i64(&world, text).xpect_eq(3);

	// reactive: the build root's component edit reaches the text
	world
		.entity_mut(container)
		.get_mut::<Slider>()
		.unwrap()
		.value = 8;
	world.update_local();
	read_i64(&world, text).xpect_eq(8);
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
	let container = parse_bsx(
		&mut world,
		None,
		"<input value=@entity:Router::Slider.value/>",
	);
	let input = world.entity(container).get::<Children>().unwrap()[0];
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
	world.entity_mut(container).insert(ChildOf(router));
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
fn reserved_ref_shadow_errors() {
	let mut world = world();
	let nodes =
		parse_document("<div bx:ref=\"Router\"/>", &BsxParseConfig::bsx())
			.unwrap();
	world
		.spawn_template(BsxTemplate::container(
			nodes,
			BsxTemplateRegistry::default(),
		))
		.map(|entity| entity.id())
		.unwrap_err()
		.to_string()
		.xpect_contains("reserved");
}

#[beet_core::test]
fn binding_prop_text() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section>{@prop:title}</section>")
		.unwrap();
	world.insert_resource(registry);
	let card = spawn_bsx(&mut world, "<Card title=\"hi\"/>");
	world.update_local();
	render_html(&mut world, card).xpect_contains("hi");
}

#[beet_core::test]
fn binding_prop_doc_bound_reactively() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section>{@prop:title}</section>")
		.unwrap();
	world.insert_resource(registry);
	let doc = world.spawn(Document::new(val!({ "name": "Alice" }))).id();
	let card =
		spawn_bsx_under(&mut world, Some(doc), "<Card title=@doc:name/>");
	world.update_local();
	world.update_local();
	render_html(&mut world, card).xpect_contains("Alice");

	world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
		val!({ "name": "Bob" });
	world.update_local();
	world.update_local();
	render_html(&mut world, card).xpect_contains("Bob");
}

#[beet_core::test]
fn binding_prop_res_bound_reactively() {
	let mut world = theme_world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section>{@prop:level}</section>")
		.unwrap();
	world.insert_resource(registry);
	let card = spawn_bsx(&mut world, "<Card level=@res:Theme.contrast/>");
	world.update_local();
	world.update_local();
	render_html(&mut world, card).xpect_contains("5");

	world.resource_mut::<Theme>().contrast = 9;
	world.update_local();
	world.update_local();
	render_html(&mut world, card).xpect_contains("9");
}

/// A user `bx:scope` above the tag must not reach into the template's props
/// store (regression: the scope walk prefixed `@prop:title` to `scope.title`,
/// silently missing the store's root `title`).
#[beet_core::test]
fn binding_prop_unaffected_by_user_scope() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Card", "<section>{@prop:title}</section>")
		.unwrap();
	world.insert_resource(registry);
	let card = spawn_bsx(
		&mut world,
		"<div bx:scope=\"user\"><Card title=\"hi\"/></div>",
	);
	world.update_local();
	world.update_local();
	render_html(&mut world, card).xpect_contains("hi");
}

#[beet_core::test]
fn binding_prop_passes_through_nested_templates() {
	let mut world = world();
	let mut registry = BsxTemplateRegistry::default();
	registry
		.insert_source("Inner", "<span>{@prop:title}</span>")
		.unwrap();
	registry
		.insert_source("Outer", "<div><Inner title=@prop:title/></div>")
		.unwrap();
	world.insert_resource(registry);
	let outer = spawn_bsx(&mut world, "<Outer title=\"hello\"/>");
	world.update_local();
	world.update_local();
	world.update_local();
	render_html(&mut world, outer).xpect_contains("hello");
}

#[beet_core::test]
fn component_tag_binding_doc() {
	let mut world = world();
	register::<Slider>(&mut world);
	let doc = world.spawn(Document::new(val!({ "level": 7i64 }))).id();
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
	world
		.entity(div)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("div");
}

/// Parse HTML-mode bytes into a container, returning the container entity.
fn parse_bsx_html(world: &mut World, bytes: &MediaBytes) -> Entity {
	let mut entity = world.spawn_empty();
	BsxParser::html()
		.parse(ParseContext::new(&mut entity, bytes))
		.unwrap();
	entity.id()
}
