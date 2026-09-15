//! The keyboard half of the focus model: the terminal's keys routed to the
//! focused entity.
//!
//! `Tab`/`Shift+Tab` move [`Focus`] through the focusables in document order
//! ([`tab_focus`]), `Enter` activates the focused element through the click
//! path ([`activate_focused_on_enter`]), and typing edits the focused
//! [`Value`] ([`write_focus_input`]). All of it reads bevy's [`KeyboardInput`]
//! stream, which is why it sits behind the `keyboard` feature: a browser's
//! native controls own their text, caret and IME, so the web composes the
//! [`focus`](super::focus) model without this half.
//!
//! Text entry knows nothing about [`Document`](beet_core::prelude::Document)
//! or [`FieldRef`](beet_core::prelude::FieldRef): it only writes
//! `Changed<Value>` on the focused entity. The bidi sync chain carries that
//! change into the document.
use super::focus::*;
use crate::prelude::PointerDown;
use crate::prelude::PointerUp;
use crate::prelude::SurfaceQuery;
use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;

/// System: pressing Enter on the focused element activates it, by firing
/// `PointerDown`+`PointerUp` on it.
///
/// Keyboard activation reuses the click path, so a focused `<button>` runs its
/// `bx:click` script and a focused `<a>` navigates, with no separate keyboard
/// wiring. Fires only on a focused element (a plain text input gets no `<button>`
/// activation; Enter in a text field is handled by the text-entry system).
pub(super) fn activate_focused_on_enter(
	mut keys: MessageReader<KeyboardInput>,
	focused: Query<Entity, With<Focus>>,
	surfaces: SurfaceQuery,
	mut commands: Commands,
) {
	// the surfaces (windows) Enter was pressed on this frame.
	let enter_windows = keys
		.read()
		.filter(|key| {
			key.state == ButtonState::Pressed && key.logical_key == Key::Enter
		})
		.map(|key| key.window)
		.collect::<HashSet<_>>();
	if enter_windows.is_empty() {
		return;
	}
	// activate the focused element of each surface Enter landed on.
	for target in focused.iter() {
		if enter_windows
			.iter()
			.any(|window| surfaces.matches(target, *window))
		{
			// the activation reuses the click path; consumers read the target, not
			// the pointer, so the target itself stands in as the pointer entity.
			commands.entity(target).trigger(PointerDown::new(target));
			commands.entity(target).trigger(PointerUp::new(target));
		}
	}
}

/// System: move [`Focus`] to the next/previous [`Focusable`] in document order on
/// `Tab`/`Shift+Tab`, wrapping at the ends.
///
/// Tab moves focus rather than typing a tab character, so a text field never
/// receives `\t`. Document order is tree pre-order from the roots; focusables
/// with no tree position (eg standalone) trail in entity order so the ring is
/// still stable.
pub(super) fn tab_focus(
	mut keys: MessageReader<KeyboardInput>,
	focusables: Query<Entity, With<Focusable>>,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
	surfaces: SurfaceQuery,
	focused: Query<Entity, With<Focus>>,
	mut commands: Commands,
) {
	// per surface (window): the net tab count and whether Shift was held. The
	// terminal bridge emits Shift+Tab as a ShiftLeft press bracketing the Tab
	// press, so both land in the same frame's stream for that window.
	let mut per_window = HashMap::<Entity, (i32, bool)>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		let entry = per_window.entry(key.window).or_default();
		match &key.logical_key {
			Key::Tab => entry.0 += 1,
			Key::Shift => entry.1 = true,
			_ => {}
		}
	}

	let full_order = focusables_in_order(&focusables, &children, &parents);
	for (window, (tabs, shift)) in per_window {
		if tabs == 0 {
			continue;
		}
		let direction = if shift { -tabs } else { tabs };
		// the focusables on this surface, in document order.
		let order = full_order
			.iter()
			.copied()
			.filter(|entity| surfaces.matches(*entity, window))
			.collect::<Vec<_>>();
		if order.is_empty() {
			continue;
		}
		// the element currently focused on this surface, if any.
		let current = focused
			.iter()
			.find(|entity| surfaces.matches(*entity, window));
		let next =
			match current.and_then(|c| order.iter().position(|&e| e == c)) {
				// wrap forward/back around the focusable ring
				Some(idx) => {
					((idx as i32 + direction).rem_euclid(order.len() as i32))
						as usize
				}
				// nothing focused yet: start at the first
				None => 0,
			};
		commands.entity(order[next]).insert(Focus);
	}
}

/// The focusables in document (tree pre-order) order, roots sorted by entity for
/// stability. A focusable with no [`ChildOf`] is its own root.
fn focusables_in_order(
	focusables: &Query<Entity, With<Focusable>>,
	children: &Query<&Children>,
	parents: &Query<&ChildOf>,
) -> Vec<Entity> {
	let is_focusable: HashSet<Entity> = focusables.iter().collect();
	// roots: ancestors-most entity of each focusable (walk up ChildOf).
	let mut roots: Vec<Entity> = is_focusable
		.iter()
		.map(|&entity| {
			let mut root = entity;
			while let Ok(child_of) = parents.get(root) {
				root = child_of.parent();
			}
			root
		})
		.collect::<HashSet<_>>()
		.into_iter()
		.collect();
	roots.sort();

	// pre-order each root, collecting focusables in document order.
	let mut order = Vec::new();
	for root in roots {
		let mut stack = vec![root];
		while let Some(entity) = stack.pop() {
			if is_focusable.contains(&entity) {
				order.push(entity);
			}
			if let Ok(child_list) = children.get(entity) {
				stack.extend(child_list.iter().rev());
			}
		}
	}
	order
}

/// Turns buffered key presses into text edits on the focused entity's [`Value`],
/// scoped per surface so each session types into its own focused field.
///
/// Only acts on `ButtonState::Pressed` (repeats flow through so held keys
/// repeat). With no focused entity, no `Value`, or no editing keys this turn,
/// it is a no-op and never marks `Changed`. A key's edits reach the focused
/// element whose surface matches the key's `window`. `pub` so consumers can
/// order against it (eg form submit runs after, so a one-frame input batch's
/// chars land before its Enter gathers them).
pub(crate) fn write_focus_input(
	mut keys: MessageReader<KeyboardInput>,
	mut focused: Query<(Entity, &mut Value, Option<&Element>), With<Focus>>,
	surfaces: SurfaceQuery,
) {
	// collect editing keys grouped by their source surface (window).
	let mut edits_by_window = HashMap::<Entity, Vec<KeyEdit>>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		let edit = match &key.logical_key {
			// normal typing, terminals also map space to a ' ' character
			Key::Character(chars) => Some(KeyEdit::Insert(chars.to_string())),
			// some backends send space distinctly
			Key::Space => Some(KeyEdit::Insert(" ".to_string())),
			Key::Backspace => Some(KeyEdit::Backspace),
			// forward delete: drops the first character in this append-at-end model
			Key::Delete => Some(KeyEdit::Delete),
			// Enter, arrows, Tab, Escape, etc belong to navigation/shortcuts
			_ => None,
		};
		if let Some(edit) = edit {
			edits_by_window.entry(key.window).or_default().push(edit);
		}
	}
	if edits_by_window.is_empty() {
		return;
	}

	// apply each surface's edits to its own focused element. A `<select>`'s
	// value is a chosen option, never typed text: typing on one refines its
	// dropdown instead (the charcell `SelectPlugin`).
	for (entity, value, element) in focused.iter_mut() {
		if element.is_some_and(|element| element.tag() == "select") {
			continue;
		}
		let edits = edits_by_window
			.iter()
			.filter(|(window, _)| surfaces.matches(entity, **window))
			.flat_map(|(_, edits)| edits.iter().cloned())
			.collect::<Vec<_>>();
		if !edits.is_empty() {
			apply_text_edits(value, edits);
		}
	}
}

/// Apply text `edits` to a focused [`Value`] through change-detection bypass, so a
/// non-text sink or no-op never dirties `Changed`; flags `Changed` once if an edit
/// actually landed.
fn apply_text_edits(mut value: Mut<Value>, edits: Vec<KeyEdit>) {
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
#[derive(Clone)]
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
	use crate::prelude::RenderSurface;
	use bevy::input::InputPlugin;

	/// Builds an [`App`] with the focus and input messaging wired up.
	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((InputPlugin, FocusPlugin));
		app
	}

	/// A pressed [`KeyboardInput`] for `key`, tagged with its source `window`.
	fn press(window: Entity, key: Key) -> KeyboardInput {
		KeyboardInput {
			key_code: bevy::input::keyboard::KeyCode::KeyA,
			logical_key: key,
			state: ButtonState::Pressed,
			text: None,
			repeat: false,
			window,
		}
	}

	/// Sends each `key` as a press from `window` and runs one frame.
	fn type_keys(
		app: &mut App,
		window: Entity,
		keys: impl IntoIterator<Item = Key>,
	) {
		for key in keys {
			app.world_mut().write_message(press(window, key));
		}
		app.update();
	}

	/// Spawn `bundle` as a focusable scoped to a fresh one-surface window,
	/// returning `(window, entity)`. The pure-mechanism focus tests have no page
	/// tree, so they tag the focusable with its own [`RenderSurface`] to receive
	/// that window's scoped input.
	fn on_surface(app: &mut App, bundle: impl Bundle) -> (Entity, Entity) {
		let window = app.world_mut().spawn_empty().id();
		let entity =
			app.world_mut().spawn((bundle, RenderSurface(window))).id();
		(window, entity)
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

	/// Whether `entity` currently holds [`Focus`].
	fn is_focused(app: &App, entity: Entity) -> bool {
		app.world().entity(entity).contains::<Focus>()
	}

	#[beet_core::test]
	fn typing_appends() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::str("")));
		type_keys(&mut app, window, [char_key("h"), char_key("i")]);
		value_of(&app, entity).xpect_eq(Value::str("hi"));
	}

	/// Two surfaces type independently: a key from surface A's window edits
	/// only A's field, never B's (the multi-tenant typing invariant).
	#[beet_core::test]
	fn typing_routes_to_its_own_surface() {
		let mut app = app();
		// two surfaces (windows), each a RenderSurface page root with a field.
		let window_a = app.world_mut().spawn_empty().id();
		let window_b = app.world_mut().spawn_empty().id();
		let field_a = app.world_mut().spawn((Focusable, Value::str(""))).id();
		let field_b = app.world_mut().spawn((Focusable, Value::str(""))).id();
		app.world_mut()
			.spawn(RenderSurface(window_a))
			.add_child(field_a);
		app.world_mut()
			.spawn(RenderSurface(window_b))
			.add_child(field_b);
		// focus both: on different surfaces, so both keep Focus.
		app.world_mut().entity_mut(field_a).insert(Focus);
		app.world_mut().entity_mut(field_b).insert(Focus);
		app.update();
		is_focused(&app, field_a).xpect_true();
		is_focused(&app, field_b).xpect_true();

		// type into surface A only
		app.world_mut().write_message(KeyboardInput {
			key_code: bevy::input::keyboard::KeyCode::KeyX,
			logical_key: char_key("x"),
			state: ButtonState::Pressed,
			text: None,
			repeat: false,
			window: window_a,
		});
		app.update();
		value_of(&app, field_a).xpect_eq(Value::str("x"));
		value_of(&app, field_b).xpect_eq(Value::str(""));
	}

	#[beet_core::test]
	fn backspace_pops() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::str("hi")));
		type_keys(&mut app, window, [Key::Backspace]);
		value_of(&app, entity).xpect_eq(Value::str("h"));
		// backspace on empty stays Str(""), does not revert to Null
		type_keys(&mut app, window, [Key::Backspace, Key::Backspace]);
		value_of(&app, entity).xpect_eq(Value::str(""));
	}

	#[beet_core::test]
	fn null_coercion() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::Null));
		type_keys(&mut app, window, [char_key("x")]);
		value_of(&app, entity).xpect_eq(Value::str("x"));
	}

	#[beet_core::test]
	fn typing_digit_into_number() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::Int(5)));
		// number fields stringify, edit, and parse back, preserving the variant
		type_keys(&mut app, window, [char_key("3")]);
		value_of(&app, entity).xpect_eq(Value::Int(53));
	}

	#[beet_core::test]
	fn invalid_number_edit_rejected() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::Int(5)));
		app.update(); // clear the spawn-time Changed tick
		// a non-numeric key leaves the number untouched and unmarked
		type_keys(&mut app, window, [char_key("x")]);
		value_of(&app, entity).xpect_eq(Value::Int(5));
		changed_count(&mut app).xpect_eq(0);
	}

	#[beet_core::test]
	fn delete_drops_first() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::str("hi")));
		type_keys(&mut app, window, [Key::Delete]);
		value_of(&app, entity).xpect_eq(Value::str("i"));
		// delete on empty stays Str(""), does not panic
		type_keys(&mut app, window, [Key::Delete, Key::Delete]);
		value_of(&app, entity).xpect_eq(Value::str(""));
	}

	#[beet_core::test]
	fn no_value_is_noop() {
		let mut app = app();
		app.world_mut().spawn(Focus);
		// typing without a Value on the focused entity must not panic
		type_keys(&mut app, Entity::PLACEHOLDER, [char_key("x")]);
	}

	#[beet_core::test]
	fn no_focus_is_noop() {
		let mut app = app();
		app.world_mut().spawn(Value::str(""));
		// typing with nothing focused must not panic
		type_keys(&mut app, Entity::PLACEHOLDER, [char_key("x")]);
	}

	#[beet_core::test]
	fn ignored_keys_dont_dirty() {
		let mut app = app();
		let (window, entity) = on_surface(&mut app, (Focus, Value::str("hi")));
		app.update(); // clear the spawn-time Changed tick
		type_keys(&mut app, window, [Key::ArrowLeft]);
		value_of(&app, entity).xpect_eq(Value::str("hi"));
		changed_count(&mut app).xpect_eq(0);
	}

	/// `Tab` cycles focus forward through focusables in document order, `Shift+Tab`
	/// back, wrapping at the ends.
	#[beet_core::test]
	fn tab_cycles_focus() {
		let mut app = app();
		// focusables in a tree so document order is the children order (not entity
		// id order), matching a real page. The tree root carries the surface, so tab
		// traversal scopes to this window.
		let first = app.world_mut().spawn(Focusable).id();
		let second = app.world_mut().spawn(Focusable).id();
		let window = app.world_mut().spawn_empty().id();
		app.world_mut()
			.spawn(RenderSurface(window))
			.add_children(&[first, second]);
		app.update();

		// no focus yet: Tab focuses the first
		type_keys(&mut app, window, [Key::Tab]);
		is_focused(&app, first).xpect_true();
		// Tab again: second
		type_keys(&mut app, window, [Key::Tab]);
		is_focused(&app, second).xpect_true();
		// Tab wraps back to first
		type_keys(&mut app, window, [Key::Tab]);
		is_focused(&app, first).xpect_true();
		// Shift+Tab goes back, wrapping to the last
		type_keys(&mut app, window, [Key::Shift, Key::Tab]);
		is_focused(&app, second).xpect_true();
	}
}
