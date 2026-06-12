//! The default BSX event/verb registration.
//!
//! Core keeps the [`EventRegistry`]/[`VerbRegistry`] empty: it knows no concrete
//! event or verb, and bevy picking never enters it. This plugin supplies the
//! concrete `click` event installer (a [`PointerDown`] observer) plus the
//! example verb set (`increment`/`decrement`/`toggle`), so every existing
//! `bx:click=verb#field` keeps working. An app that wants a different vocabulary
//! registers its own instead of (or alongside) this default set.
use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the default BSX event/verb vocabulary into the core seam, plus the
/// widget set by name so a `<Head/>`/`<Sidebar/>` tag resolves.
///
/// Builds on [`BsxPlugin`] (which seeds the empty registries): the `click`
/// installer wires a [`PointerDown`] observer that runs the bound verb with
/// exclusive world access, and the example verbs mutate the bound field's
/// [`Value`].
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
/// fire, runs the bound verb against the target through an exclusive command.
fn register_default_events(world: &mut World) {
	world.resource_mut::<EventRegistry>().insert(
		"click",
		|entity: &mut EntityWorldMut, verb: SmolStr, _field: FieldPath| {
			entity.observe(
				move |ev: On<PointerDown>, mut commands: Commands| {
					let target = ev.target;
					let verb = verb.clone();
					// run the verb with exclusive world access, never inline in the
					// observer: a verb may need to read/write beyond the target's Value.
					commands.queue(move |world: &mut World| {
						if let Some(verb) =
							world.resource::<VerbRegistry>().get(&verb)
						{
							verb(&mut world.entity_mut(target));
						}
					});
				},
			);
		},
	);
}

/// Register the example verb set, each mutating the target's bound [`Value`].
fn register_default_verbs(world: &mut World) {
	let mut verbs = world.resource_mut::<VerbRegistry>();
	verbs.insert("increment", |entity: &mut EntityWorldMut| {
		if let Some(mut value) = entity.get_mut::<Value>() {
			*value = Value::Int(value.as_i64().unwrap_or(0) + 1);
		}
	});
	verbs.insert("decrement", |entity: &mut EntityWorldMut| {
		if let Some(mut value) = entity.get_mut::<Value>() {
			*value = Value::Int(value.as_i64().unwrap_or(0) - 1);
		}
	});
	verbs.insert("toggle", |entity: &mut EntityWorldMut| {
		if let Some(mut value) = entity.get_mut::<Value>() {
			*value = Value::Bool(!matches!(*value, Value::Bool(true)));
		}
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn click_runs_verb() {
		let mut world =
			(BsxDefaultsPlugin, DocumentPlugin).into_world();
		let binding = EventBinding {
			event: "click".into(),
			verb: "increment".into(),
			field: FieldPath::new(["count"]),
			init: None,
		};
		let entity = {
			let mut entity = world.spawn(Value::Int(0));
			install_event(&mut entity, &binding);
			entity.id()
		};
		// fire the trigger; the queued command runs the verb on flush.
		world.entity_mut(entity).trigger(PointerDown::new(entity));
		world.flush();
		world.get::<Value>(entity).unwrap().xpect_eq(Value::Int(1));
	}
}
