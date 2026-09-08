//! The concrete `bx:<event>` installer: a trigger that runs the directive's
//! script against the live world.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Binds the event-target entity to `target` before the authored source runs.
///
/// The one line of scaffolding an event script gets: `world.entity(id)` is the
/// JS face of an [`AsyncEntity`], so `target.get_field`/`target.set_field`
/// resolve through the same ancestor-document walk and [`DocumentScope`] prefix
/// a display binding reads through. That is what keeps the counter two lines
/// and stops it ever naming a document.
const EVENT_PRELUDE: &str = "const target = world.entity(input.target);";

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
			let script = Script::<Value, Value>::new(format!(
				"{EVENT_PRELUDE}\n{script}"
			));
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

	/// The two-line counter, the whole point of the phase: a click reads and
	/// writes a document field without the markup ever naming a document.
	#[beet_core::test]
	async fn click_increments_a_document_field() {
		let mut world = world_ext::ui_world();
		let doc = world.spawn(Document::new(value!({ "count": 0 }))).id();
		let button = click_target(
			&mut world,
			doc,
			r#"<button bx:click="
				const count = await target.get_field('count');
				await target.set_field('count', count + 1);
			">+</button>"#,
		);
		click(&mut world, button).await;
		count(&world, doc, "count").xpect_eq(1);
		click(&mut world, button).await;
		count(&world, doc, "count").xpect_eq(2);
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
			r#"<div bx:scope="counter"><button bx:click="await target.set_field('count', 7)">+</button></div>"#,
		)
		.xmap(|div| world.entity(div).get::<Children>().unwrap()[0]);
		click(&mut world, button).await;
		count(&world, doc, "counter.count").xpect_eq(7);
	}

	/// A script is the whole vocabulary, so a behavior nothing in rust knows
	/// about is authored in the document: this one branches and writes a string.
	#[beet_core::test]
	async fn a_script_expresses_its_own_behavior() {
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
