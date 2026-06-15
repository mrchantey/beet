//! The default BSX event/verb registration.
//!
//! Core keeps the [`EventRegistry`]/[`VerbRegistry`] empty: it knows no concrete
//! event or verb, and bevy picking never enters it. This plugin supplies the
//! concrete `click` event installer (a [`PointerDown`] observer) plus the
//! example verb set (`increment`/`decrement`/`toggle`/`set`), so every
//! `bx:click=increment{ field: @doc:count }` keeps working. An app that wants a
//! different vocabulary registers its own instead of (or alongside) this set.
use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the default BSX event/verb vocabulary into the core seam, plus the
/// widget set by name so a `<Head/>`/`<Sidebar/>` tag resolves.
///
/// Builds on [`BsxPlugin`] (which seeds the empty registries): the `click`
/// installer wires a [`PointerDown`] observer that runs the bound verb with
/// exclusive world access and its resolved [`VerbArgs`], and the example verbs
/// mutate a document field through its binding argument's field helper.
#[derive(Default)]
pub struct BsxDefaultsPlugin;

impl Plugin for BsxDefaultsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((BsxPlugin, widget_plugin));
		register_default_events(app.world_mut());
		register_default_verbs(app.world_mut());
	}
}

/// Register the `click` event installer: a [`PointerDown`] observer that, on
/// fire, runs the bound verb with its arguments against the host through an
/// exclusive command.
fn register_default_events(world: &mut World) {
	world.resource_mut::<EventRegistry>().insert(
		"click",
		|entity: &mut EntityWorldMut, verb: SmolStr, args: VerbArgs| {
			entity.observe(
				move |ev: On<PointerDown>, mut commands: Commands| {
					let target = ev.target;
					let verb = verb.clone();
					let args = args.clone();
					// run the verb with exclusive world access, never inline in the
					// observer: a verb may read/write the document or a resource.
					commands.queue(move |world: &mut World| {
						if let Some(verb) =
							world.resource::<VerbRegistry>().get(&verb)
						{
							verb(&mut world.entity_mut(target), &args);
						}
					});
				},
			);
		},
	);
}

/// Register the example verb set: each mutates a document field through its
/// `field` binding argument's read-modify-write helper.
///
/// Each verb passes its `js_verb`, the self-contained JS twin the renderer emits
/// into `data-bx-verbs` and the thin-client runtime installs (the runtime ships
/// zero built-in verbs). Every twin matches the Rust semantics exactly.
fn register_default_verbs(world: &mut World) {
	// computed before borrowing the registry; `+`/`-` are the only difference.
	let increment_js = counter_js('+');
	let decrement_js = counter_js('-');
	let mut verbs = world.resource_mut::<VerbRegistry>();
	// `increment{ field, amount: i64 = 1 }`: add `amount` to the bound field.
	verbs.insert(
		"increment",
		VerbSchema::new()
			.binding("field")
			.optional_value("amount", ValueSchema::of::<i64>(), Value::Int(1)),
		Some(increment_js.as_str()),
		|entity: &mut EntityWorldMut, args: &VerbArgs| {
			let amount = args
				.value("amount")
				.and_then(|value| value.as_i64().ok())
				.unwrap_or(1);
			update_field(entity, args, |value| {
				*value = Value::Int(value.as_i64().unwrap_or(0) + amount)
			});
		},
	);
	// `decrement{ field, amount: i64 = 1 }`: subtract `amount` from the field.
	verbs.insert(
		"decrement",
		VerbSchema::new()
			.binding("field")
			.optional_value("amount", ValueSchema::of::<i64>(), Value::Int(1)),
		Some(decrement_js.as_str()),
		|entity: &mut EntityWorldMut, args: &VerbArgs| {
			let amount = args
				.value("amount")
				.and_then(|value| value.as_i64().ok())
				.unwrap_or(1);
			update_field(entity, args, |value| {
				*value = Value::Int(value.as_i64().unwrap_or(0) - amount)
			});
		},
	);
	// `toggle{ field }`: flip the bound boolean field (true only when not true).
	verbs.insert(
		"toggle",
		VerbSchema::new().binding("field"),
		Some("entity.set_field(args.field, entity.get_field(args.field) !== true);"),
		|entity: &mut EntityWorldMut, args: &VerbArgs| {
			update_field(entity, args, |value| {
				*value = Value::Bool(!matches!(value, Value::Bool(true)))
			});
		},
	);
	// `set{ field, value }`: write `value` to the bound field.
	verbs.insert(
		"set",
		VerbSchema::new().binding("field").value("value", ValueSchema::Any),
		Some("entity.set_field(args.field, args.value);"),
		|entity: &mut EntityWorldMut, args: &VerbArgs| {
			let Some(new_value) = args.value("value").cloned() else {
				return;
			};
			update_field(entity, args, move |value| *value = new_value);
		},
	);
}

/// The JS twin body of `increment`/`decrement`: coerce the bound field to a
/// number (non-number reads as 0, matching `as_i64().unwrap_or(0)`), then apply
/// `op` with `amount` (default 1). Self-contained: it touches only
/// `entity`/`args`, as `new Function("entity", "args", body)` requires.
fn counter_js(op: char) -> String {
	format!(
		"const value = entity.get_field(args.field);\n\
		 const current = typeof value === \"number\" ? value : 0;\n\
		 const amount = args.amount == null ? 1 : args.amount;\n\
		 entity.set_field(args.field, current {op} amount);"
	)
}

/// Read-modify-write the `field` binding argument against the host, the shared
/// shape of every default verb (a graceful no-op when `field` is absent).
fn update_field(
	entity: &mut EntityWorldMut,
	args: &VerbArgs,
	func: impl FnOnce(&mut Value),
) {
	if let Some(field) = args.field("field") {
		field.update(entity, func).ok();
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Spawn a `<button bx:click=verb{..}>` under `doc` and return the button
	/// entity (the container's content child the observer is wired onto).
	fn click_button(world: &mut World, doc: Entity, markup: &str) -> Entity {
		let container = world
			.spawn_template(BsxTemplate::container(
				parse_document(markup, &BsxParseConfig::bsx()).unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.entity_mut(container).insert(ChildOf(doc));
		world.update_local();
		world.entity(container).get::<Children>().unwrap()[0]
	}

	#[beet_core::test]
	fn click_increments_document_field() {
		let mut world = ui_world();
		let doc = world.spawn(Document::new(val!({ "count": 0 }))).id();
		let button = click_button(
			&mut world,
			doc,
			"<button bx:click=increment{ field: @doc:count }>+</button>",
		);
		// fire the trigger; the queued command runs the verb on flush.
		world.entity_mut(button).trigger(PointerDown::new(button));
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
		let mut world = ui_world();
		let doc = world.spawn(Document::new(val!({ "count": 0 }))).id();
		let button = click_button(
			&mut world,
			doc,
			"<button bx:click=increment{ field: @doc:count, amount: 3 }>+</button>",
		);
		world.entity_mut(button).trigger(PointerDown::new(button));
		world.flush();
		world.entity_mut(button).trigger(PointerDown::new(button));
		world.flush();
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<i64>(&[FieldSegment::key("count")])
			.unwrap()
			.xpect_eq(6);
	}

	#[beet_core::test]
	fn set_writes_document_field() {
		let mut world = ui_world();
		let doc = world.spawn(Document::new(val!({ "status": "pending" }))).id();
		let button = click_button(
			&mut world,
			doc,
			r#"<button bx:click=set{ field: @doc:status, value: "done" }>ok</button>"#,
		);
		world.entity_mut(button).trigger(PointerDown::new(button));
		world.flush();
		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldSegment::key("status")])
			.unwrap()
			.xpect_eq("done");
	}
}
