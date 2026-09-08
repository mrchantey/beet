//! The concrete `bx:<event>` installer: a trigger that runs the directive's
//! script against the live world.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Wrap an authored event script so it runs as an *expression* where it is one,
/// and as a statement body otherwise.
///
/// Two things are scaffolded. `target` is the event-target entity, bound as an
/// ordinary lexical `const`: `world.entity(id)` is the JS face of an
/// [`AsyncEntity`], so `target.get_field`/`set_field`/`with_field` resolve
/// through the same ancestor-document walk and [`DocumentScope`] prefix a
/// display binding reads through, which is what stops a handler ever naming a
/// document.
///
/// The second is the expression form, and it is why the counter needs no
/// `await`. An expression becomes the arrow's *body*, so its value (the promise
/// any `world` call returns) is what the run resolves to and the host awaits it:
/// `bx:click="target.with_field('count', count => count + 1)"` still reports its
/// errors rather than leaving a floating promise. Anything that is not an
/// expression fails to parse and falls back to the statement form, where
/// `await` is legal exactly as before. The engine makes the choice, not us, so
/// the two forms cannot drift from what JavaScript actually accepts; a lexical
/// heuristic (a `;` or a newline means body, the shape Vue uses) is the cheaper
/// option if the double parse ever proves to be a problem.
///
/// `eval` is *direct*, so the authored source sees `target` as if it had been
/// written here, and the trailing newline before each closer keeps a source
/// ending in a `//` comment from swallowing it.
///
/// # Errors
/// Errors only when the authored source cannot be encoded as a JS string.
fn wrap_event_script(script: &str) -> Result<String> {
	let source = serde_json::to_string(script)?;
	format!(
		r#"const target = world.entity(input.target);
const source = {source};
let result;
try {{
	result = eval("(async () => (" + source + "\n))()");
}} catch (err) {{
	if (!(err instanceof SyntaxError)) throw err;
	result = eval("(async () => {{" + source + "\n}})()");
}}
return await result;"#
	)
	.xok()
}

/// Register the `click` event installer: a [`PointerDown`] observer that, on
/// fire, evaluates the directive's script.
///
/// What the script may reach is the host's own [`ScriptConfig`] when it carries
/// one, so narrowing a button's grant is an ordinary spread
/// (`<button bx:click=".." {ScriptConfig{write:["Document"]}}>`); absent one it
/// is [`ScriptConfig::default`], world-capable and unrestricted, which is where
/// the phase leaves it.
pub(super) fn register_event_scripts(world: &mut World) {
	world.resource_mut::<EventRegistry>().insert(
		"click",
		|entity: &mut EntityWorldMut, script: &str| {
			let wrapped = match wrap_event_script(script) {
				Ok(wrapped) => wrapped,
				Err(err) => {
					error!("`bx:click`: {err}");
					return;
				}
			};
			let script = Script::<Value, Value>::new(wrapped);
			entity.observe(
				move |ev: On<PointerDown>, mut commands: Commands| {
					let (script, target) = (script.clone(), ev.target);
					// queued rather than run inline: the evaluation needs the world,
					// and every `world` call it makes is served at a sync point.
					commands.queue(move |world: &mut World| {
						let Ok(host) = world.get_entity(target) else {
							return;
						};
						let config = host
							.get::<ScriptConfig>()
							.cloned()
							.unwrap_or_default();
						// the event as a value: its target now, room for a payload
						// (a key, a form value) as more triggers are registered.
						let mut input = Map::default();
						input.insert("target", entity_id::encode(target));
						let input = Value::Map(input);
						world.run_async_local(move |world| async move {
							script.run(input, world, &config).await.map(|_| ())
						});
					});
				},
			);
		},
	);
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Spawn `markup` under `doc` and return the element the observer is wired
	/// onto (the container's one content child).
	fn click_target(world: &mut World, doc: Entity, markup: &str) -> Entity {
		let container = world
			.spawn_template(BsxTemplate::container(
				BsxNode::parse_document(markup, &BsxParseConfig::bsx())
					.unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.entity_mut(container).insert(ChildOf(doc));
		world.update_local();
		world.entity(container).get::<Children>().unwrap()[0]
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
			r#"<button bx:click="target.with_field('count', count => count + 1)">+</button>"#,
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
			r#"<button bx:click="target.with_field('user', user => { user.name = 'ada lovelace' })">rename</button>"#,
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
			r#"<div bx:scope="counter"><button bx:click="target.set_field('count', 7)">+</button></div>"#,
		)
		.xmap(|div| world.entity(div).get::<Children>().unwrap()[0]);
		click(&mut world, button).await;
		count(&world, doc, "counter.count").xpect_eq(7);
	}

	/// A source that is not an expression falls back to the statement form,
	/// where `await` is legal and needed: the pump stops the moment the script's
	/// own promise settles, so an unawaited call in a body is a write that never
	/// lands. A script is also the whole vocabulary, so this behavior (branch on
	/// the current value, write a string) is one nothing in rust knows about.
	#[beet_core::test]
	async fn a_multi_statement_script_runs_as_a_body() {
		let mut world = world_ext::ui_world();
		let doc = world
			.spawn(Document::new(value!({ "status": "pending" })))
			.id();
		let button = click_target(
			&mut world,
			doc,
			r#"<button bx:click="
				const status = await target.get_field('status');
				await target.set_field('status', status === 'done' ? 'pending' : 'done');
			">ok</button>"#,
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
}
