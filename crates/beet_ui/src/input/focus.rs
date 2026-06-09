//! Keyboard input routed to the single focused entity's [`Value`].
//!
//! This layer knows nothing about [`Document`](beet_core::prelude::Document) or
//! [`FieldRef`](beet_core::prelude::FieldRef): it only writes `Changed<Value>` on
//! the focused entity. The bidi sync chain carries that change into the
//! document. Consumers read it with
//! `Query<&Value, (Changed<Value>, With<Focus>)>`.
use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;

/// Marker for the single focused entity that receives keyboard input.
///
/// At most one entity carries `Focus` at a time. The `on_add` hook
/// enforces this by clearing `Focus` from every other entity, so the
/// invariant holds regardless of who sets focus. Having no focused
/// entity is a valid steady state.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
#[component(on_add = Self::on_add)]
pub struct Focus;

impl Focus {
	/// Clears `Focus` from every other entity so only the newest one keeps it.
	///
	/// A [`DeferredWorld`] cannot run an arbitrary query inline, so the
	/// full-world work is queued as a command closure.
	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let added = cx.entity;
		world.commands().queue(move |world: &mut World| {
			let stale = world
				.query_filtered::<Entity, With<Focus>>()
				.iter(world)
				.filter(|entity| *entity != added)
				.collect::<Vec<_>>();
			for entity in stale {
				world.entity_mut(entity).remove::<Focus>();
			}
		});
	}
}

/// Registers the focus model and the keyboard-to-[`Value`] system.
///
/// Backend-agnostic: whoever assembles the app adds this alongside the
/// renderer plugins. `write_focus_input` runs in `Update`, after each
/// backend's input collection has buffered [`KeyboardInput`] messages.
pub struct FocusPlugin;

impl Plugin for FocusPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Focus>()
			.add_systems(Update, write_focus_input);
	}
}

/// Turns buffered key presses into text edits on the focused entity's [`Value`].
///
/// Only acts on `ButtonState::Pressed` (repeats flow through so held keys
/// repeat). With no focused entity, no `Value`, or no editing keys this turn,
/// it is a no-op and never marks `Changed`.
fn write_focus_input(
	mut keys: MessageReader<KeyboardInput>,
	mut focused: Query<&mut Value, With<Focus>>,
) {
	// collect editing keys first so a non-editing turn never touches the query
	let edits = keys
		.read()
		.filter(|key| key.state == ButtonState::Pressed)
		.filter_map(|key| match &key.logical_key {
			// normal typing, terminals also map space to a ' ' character
			Key::Character(chars) => Some(KeyEdit::Insert(chars.to_string())),
			// some backends send space distinctly
			Key::Space => Some(KeyEdit::Insert(" ".to_string())),
			Key::Backspace => Some(KeyEdit::Backspace),
			// forward delete: drops the first character in this append-at-end model
			Key::Delete => Some(KeyEdit::Delete),
			// Enter, arrows, Tab, Escape, etc belong to navigation/shortcuts
			_ => None,
		})
		.collect::<Vec<_>>();
	if edits.is_empty() {
		return;
	}

	// no focused entity, or it has no Value, means no write
	let Ok(mut value) = focused.single_mut() else {
		return;
	};

	// edit through bypass so a non-text sink or no-op never dirties Changed,
	// then flag Changed once if an edit actually landed
	let bypass = value.bypass_change_detection();
	let changed = edits.into_iter().fold(false, |changed, edit| {
		let did_edit = bypass
			.edit_text(|text| match edit {
				KeyEdit::Insert(chars) => text.push_str(&chars),
				KeyEdit::Backspace => {
					text.pop();
				}
				KeyEdit::Delete => {
					if !text.is_empty() {
						text.remove(0);
					}
				}
			})
			// a rejected edit (eg a non-numeric key in a number field) is a no-op
			.unwrap_or(false);
		changed || did_edit
	});
	if changed {
		value.set_changed();
	}
}

/// A single resolved keyboard edit applied to the focused text.
enum KeyEdit {
	/// Append the given characters at the end.
	Insert(String),
	/// Remove the last character.
	Backspace,
	/// Remove the first character (forward delete).
	Delete,
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::input::InputPlugin;

	/// Builds an [`App`] with the focus and input messaging wired up.
	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((InputPlugin, FocusPlugin));
		app
	}

	/// A pressed [`KeyboardInput`] for the given logical `key`.
	fn press(key: Key) -> KeyboardInput {
		KeyboardInput {
			key_code: bevy::input::keyboard::KeyCode::KeyA,
			logical_key: key,
			state: ButtonState::Pressed,
			text: None,
			repeat: false,
			window: Entity::PLACEHOLDER,
		}
	}

	/// Sends each `key` as a press and runs one frame.
	fn type_keys(app: &mut App, keys: impl IntoIterator<Item = Key>) {
		for key in keys {
			app.world_mut().write_message(press(key));
		}
		app.update();
	}

	fn char_key(text: &str) -> Key { Key::Character(text.into()) }

	/// Clones the [`Value`] currently on `entity`.
	fn value_of(app: &App, entity: Entity) -> Value {
		app.world().entity(entity).get::<Value>().unwrap().clone()
	}

	/// Counts entities whose [`Value`] is flagged `Changed` this frame.
	fn changed_count(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<Entity, Changed<Value>>()
			.iter(app.world())
			.count()
	}

	#[beet_core::test]
	fn typing_appends() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::str(""))).id();
		type_keys(&mut app, [char_key("h"), char_key("i")]);
		value_of(&app, entity).xpect_eq(Value::str("hi"));
	}

	#[beet_core::test]
	fn backspace_pops() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::str("hi"))).id();
		type_keys(&mut app, [Key::Backspace]);
		value_of(&app, entity).xpect_eq(Value::str("h"));
		// backspace on empty stays Str(""), does not revert to Null
		type_keys(&mut app, [Key::Backspace, Key::Backspace]);
		value_of(&app, entity).xpect_eq(Value::str(""));
	}

	#[beet_core::test]
	fn null_coercion() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::Null)).id();
		type_keys(&mut app, [char_key("x")]);
		value_of(&app, entity).xpect_eq(Value::str("x"));
	}

	#[beet_core::test]
	fn typing_digit_into_number() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::Int(5))).id();
		// number fields stringify, edit, and parse back, preserving the variant
		type_keys(&mut app, [char_key("3")]);
		value_of(&app, entity).xpect_eq(Value::Int(53));
	}

	#[beet_core::test]
	fn invalid_number_edit_rejected() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::Int(5))).id();
		app.update(); // clear the spawn-time Changed tick
		// a non-numeric key leaves the number untouched and unmarked
		type_keys(&mut app, [char_key("x")]);
		value_of(&app, entity).xpect_eq(Value::Int(5));
		changed_count(&mut app).xpect_eq(0);
	}

	#[beet_core::test]
	fn delete_drops_first() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::str("hi"))).id();
		type_keys(&mut app, [Key::Delete]);
		value_of(&app, entity).xpect_eq(Value::str("i"));
		// delete on empty stays Str(""), does not panic
		type_keys(&mut app, [Key::Delete, Key::Delete]);
		value_of(&app, entity).xpect_eq(Value::str(""));
	}

	#[beet_core::test]
	fn single_focus_invariant() {
		let mut app = app();
		let first = app.world_mut().spawn(Focus).id();
		let second = app.world_mut().spawn(Focus).id();
		app.update(); // flush the queued on_add command
		app.world().entity(first).contains::<Focus>().xpect_false();
		app.world().entity(second).contains::<Focus>().xpect_true();
	}

	#[beet_core::test]
	fn no_value_is_noop() {
		let mut app = app();
		app.world_mut().spawn(Focus);
		// typing without a Value on the focused entity must not panic
		type_keys(&mut app, [char_key("x")]);
	}

	#[beet_core::test]
	fn no_focus_is_noop() {
		let mut app = app();
		app.world_mut().spawn(Value::str(""));
		// typing with nothing focused must not panic
		type_keys(&mut app, [char_key("x")]);
	}

	#[beet_core::test]
	fn ignored_keys_dont_dirty() {
		let mut app = app();
		let entity = app.world_mut().spawn((Focus, Value::str("hi"))).id();
		app.update(); // clear the spawn-time Changed tick
		type_keys(&mut app, [Key::ArrowLeft]);
		value_of(&app, entity).xpect_eq(Value::str("hi"));
		changed_count(&mut app).xpect_eq(0);
	}
}
