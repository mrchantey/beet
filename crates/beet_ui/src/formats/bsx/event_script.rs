//! The concrete `bx:on<event>` handler: a `click` reaching an element runs each
//! of its `OnEvent { event: "click" }` children against the live world.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Compose an authored event script into a [`Script`] body: the host's own
/// bindings, then the authored body through [`Script::statements`].
///
/// `target` is the event-target entity, bound as an ordinary lexical `const`:
/// `world.entity(id)` is the JS face of an [`AsyncEntity`], so
/// `target.get_field`/`set_field`/`with_field` resolve through the same
/// ancestor-document walk and [`DocumentScope`] prefix a display binding reads
/// through, which is what stops a handler ever naming a document.
///
/// The prelude makes the composed source a block, so the authored body has to
/// carry its own `return` only where its *own* shape is a block. An expression
/// body needs neither `return` nor `await`, which is why the counter is one
/// line: `bx:onclick="target.with_field('count', count => count + 1)"` becomes
/// `return (..)` inside an async function, whose promise the run adopts, so the
/// write is awaited and its errors reported rather than left floating.
///
/// A script is a script: this is the same [`Script::statements`] rule an
/// `ExchangeScript` route or a `<RunScript>` leaf gets, applied through the one
/// implementation so the two cannot drift.
fn event_script_body(script: &str) -> String {
	format!(
		"{{\nconst target = world.entity(input.target);\n{}\n}}",
		Script::<Value, Value>::statements(script)
	)
}

/// Runs the `click` handlers of the entity a [`PointerDown`] reaches, once
/// per entity it propagates through, so a click on a button's label runs the
/// button's handler.
///
/// Each handler is an [`OnEvent`] child, its source read through
/// [`ElementTextQuery::script_source`] when it fires, so a handler loaded from
/// a saved scene runs as one authored in markup. What it may reach is the
/// host's own [`ScriptConfig`] when it carries one, so narrowing a button's
/// grant is an ordinary spread
/// (`<button bx:onclick=".." {ScriptConfig{write:["Document"]}}>`); absent one
/// it is [`ScriptConfig::default`], world-capable and unrestricted.
pub(super) fn run_click_scripts(
	ev: On<PointerDown>,
	children: Query<&Children>,
	events: Query<&OnEvent>,
	text: ElementTextQuery,
	configs: Query<&ScriptConfig>,
	mut commands: Commands,
) {
	let host = ev.target;
	let config = configs.get(host).cloned().unwrap_or_default();
	for handler in children.get(host).into_iter().flatten().filter(|child| {
		events
			.get(**child)
			.is_ok_and(|on_event| on_event.event == "click")
	}) {
		let script = Script::<Value, Value>::new(event_script_body(
			&text.script_source(*handler),
		));
		let config = config.clone();
		// queued rather than run inline: the evaluation needs the world, and
		// every `world` call it makes is served at a sync point.
		commands.queue(move |world: &mut World| {
			// the event as a value: its target now, room for a payload (a
			// key, a form value) as more triggers are registered.
			let mut input = Map::default();
			input.insert("target", entity_id::encode(host));
			let input = Value::Map(input);
			world.run_async_local(move |world| async move {
				script.run(input, world, &config).await.map(|_| ())
			});
		});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	/// Spawn `markup` under `doc` and return the element the observer is wired
	/// onto (the container's one content child).
	fn click_target(world: &mut World, doc: Entity, markup: &str) -> Entity {
		let body = BsxParser::body(world, markup).unwrap();
		let target = world.spawn_template(body).unwrap().id();
		world.entity_mut(target).insert(ChildOf(doc));
		world.update_local();
		target
	}

	/// Fire the trigger and drive the world until the script's evaluation has
	/// settled: the script is a task, and each of its `world` calls is one more
	/// round trip through the sync point.
	async fn click(world: &mut World, target: Entity) {
		world.entity_mut(target).trigger(PointerDown::new(target));
		AsyncRunner::settle_async_tasks(world).await;
	}

	/// The count a document holds at `path`.
	fn count(world: &World, doc: Entity, path: &str) -> i64 {
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<i64>(&FieldPath::parse(path))
			.unwrap()
	}

	/// The one-line counter, the whole point of the phase: a click reads and
	/// writes a document field without the markup ever naming a document, and
	/// without an `await`, since the source is an expression the wrapper returns.
	#[beet_core::test]
	async fn click_increments_a_document_field() {
		let mut world = world_ext::ui_world();
		let doc = world.spawn(Document::new(value!({ "count": 0 }))).id();
		let button = click_target(
			&mut world,
			doc,
			r#"<button bx:onclick="target.with_field('count', count => count + 1)">+</button>"#,
		);
		click(&mut world, button).await;
		count(&world, doc, "count").xpect_eq(1);
		click(&mut world, button).await;
		count(&world, doc, "count").xpect_eq(2);
	}

	/// `with_field` keeps the argument when the closure returns nothing, so a
	/// map or list field is edited in place rather than rebuilt.
	#[beet_core::test]
	async fn with_field_edits_a_map_in_place() {
		let mut world = world_ext::ui_world();
		let doc = world
			.spawn(Document::new(
				value!({ "user": { "name": "ada", "age": 36 } }),
			))
			.id();
		let button = click_target(
			&mut world,
			doc,
			r#"<button bx:onclick="target.with_field('user', user => { user.name = 'ada lovelace' })">rename</button>"#,
		);
		click(&mut world, button).await;
		let doc = world.entity(doc).get::<Document>().unwrap();
		doc.get_field::<String>(&FieldPath::parse("user.name"))
			.unwrap()
			.xpect_eq("ada lovelace");
		// the untouched sibling survived, so the write was an edit not a replace
		doc.get_field::<i64>(&FieldPath::parse("user.age"))
			.unwrap()
			.xpect_eq(36);
	}

	/// The field helpers resolve as a display binding does, so a `bx:scope`
	/// between the host and its document prefixes the path.
	#[beet_core::test]
	async fn a_scope_prefixes_the_written_field() {
		let mut world = world_ext::ui_world();
		let doc = world.spawn(Document::default()).id();
		let button = click_target(
			&mut world,
			doc,
			r#"<div bx:scope="counter"><button bx:onclick="target.set_field('count', 7)">+</button></div>"#,
		)
		.xmap(|div| world.entity(div).get::<Children>().unwrap()[0]);
		click(&mut world, button).await;
		count(&world, doc, "counter.count").xpect_eq(7);
	}

	/// A `{ .. }` body is a block, where `await` is legal and needed: the pump
	/// stops the moment the script's own promise settles, so an unawaited call in
	/// a block is a write that never lands. A script is also the whole
	/// vocabulary, so this behavior (branch on the current value, write a string)
	/// is one nothing in rust knows about.
	#[beet_core::test]
	async fn a_multi_statement_script_runs_as_a_body() {
		let mut world = world_ext::ui_world();
		let doc = world
			.spawn(Document::new(value!({ "status": "pending" })))
			.id();
		let button = click_target(
			&mut world,
			doc,
			r#"<button bx:onclick="{
				const status = await target.get_field('status');
				await target.set_field('status', status === 'done' ? 'pending' : 'done');
			}">ok</button>"#,
		);
		click(&mut world, button).await;
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&FieldPath::parse("status"))
			.unwrap()
			.xpect_eq("done");
		click(&mut world, button).await;
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&FieldPath::parse("status"))
			.unwrap()
			.xpect_eq("pending");
	}

	/// A handler is behaviour, never content: the button renders its label
	/// alone.
	#[beet_core::test]
	fn a_handler_renders_as_nothing() {
		let mut world = world_ext::ui_world();
		let doc = world.spawn(Document::default()).id();
		let button = click_target(
			&mut world,
			doc,
			r#"<button bx:onclick="target.set_field('count', 7)">+</button>"#,
		);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(
				&mut world,
				button,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
			.xpect_contains("+")
			.xnot()
			.xpect_contains("set_field");
	}

	/// A handler is data, so it survives a save and a reload and still runs,
	/// written explicitly as a `<script {OnEvent}>` child or as the
	/// `bx:onclick` that is sugar for one.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn a_handler_survives_a_save_and_a_reload() {
		for markup in [
			r#"<button>+<script bx:event="click">target.with_field('count', count => count + 1)</script></button>"#,
			r#"<button bx:onclick="target.with_field('count', count => count + 1)">+</button>"#,
		] {
			let mut world = world_ext::ui_world();
			let doc = world.spawn(Document::new(value!({ "count": 0 }))).id();
			let button = click_target(&mut world, doc, markup);
			let bytes = TemplateSaver::new()
				.save_roots(&mut world, MediaType::Json, [button])
				.unwrap();
			bytes
				.as_utf8()
				.unwrap()
				.xref()
				.xpect_contains("OnEvent")
				.xpect_contains("with_field");

			let mut world = world_ext::ui_world();
			let doc = world.spawn(Document::new(value!({ "count": 0 }))).id();
			let button = TemplateLoader::new(&mut world)
				.with_entity(doc)
				.load(&bytes)
				.unwrap()[0];
			click(&mut world, button).await;
			count(&world, doc, "count").xpect_eq(1);
		}
	}
}
