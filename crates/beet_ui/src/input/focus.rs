//! Keyboard focus and text entry, renderer-agnostic.
//!
//! [`Focus`] marks the single focused entity; [`Focusable`] marks elements that
//! can take focus (`<input>`/`<button>`/`<a>`/`<textarea>`, inferred from the
//! tag). Clicking a focusable focuses it ([`focus_on_click`]); `Tab`/`Shift+Tab`
//! move focus through the focusables in document order ([`tab_focus`]); the
//! focused entity carries the [`Focused`](crate::prelude::ElementState::Focused)
//! state so `:focus`/`:focus-visible` rules apply.
//!
//! Text entry ([`write_focus_input`]) knows nothing about
//! [`Document`](beet_core::prelude::Document) or
//! [`FieldRef`](beet_core::prelude::FieldRef): it only writes `Changed<Value>` on
//! the focused entity. The bidi sync chain carries that change into the document.
use crate::prelude::ElementState;
use crate::prelude::ElementStateMap;
use crate::prelude::PointerDown;
use crate::prelude::PointerUp;
use crate::prelude::PrimaryPointer;
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

/// Marks an element that can receive keyboard [`Focus`].
///
/// Auto-inferred from the tag (`<input>`/`<button>`/`<a>`/`<textarea>`) by
/// [`infer_focusable`], or inserted directly. [`tab_focus`] cycles focus through
/// focusables in document (tree) order.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
pub struct Focusable;

/// Tags that are focusable by default, mirroring the browser's sequential focus
/// navigation order.
const FOCUSABLE_TAGS: &[&str] = &["input", "button", "a", "textarea", "select"];

/// Registers the focus model, focusable inference, click-to-focus, Tab
/// traversal, the `:focus` state sync, and the keyboard-to-[`Value`] system.
///
/// Backend-agnostic: whoever assembles the app adds this alongside the renderer
/// plugins. The input systems run in `Update`, after each backend's input
/// collection has buffered [`KeyboardInput`]/[`PointerDown`].
#[derive(Default)]
pub struct FocusPlugin;

impl Plugin for FocusPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Focus>()
			.register_type::<Focusable>()
			.add_observer(infer_focusable)
			.add_observer(focus_on_click)
			.add_systems(
				Update,
				(tab_focus, activate_focused_on_enter, write_focus_input),
			)
			.add_systems(PostUpdate, sync_focus_state);
	}
}

/// System: pressing Enter on the focused element activates it, by firing
/// `PointerDown`+`PointerUp` on it.
///
/// Keyboard activation reuses the click path, so a focused `<button>` runs its
/// `bx:click` verb and a focused `<a>` navigates, with no separate keyboard
/// wiring. Fires only on a focused element (a plain text input gets no `<button>`
/// activation; Enter in a text field is handled by the text-entry system).
fn activate_focused_on_enter(
	mut keys: MessageReader<KeyboardInput>,
	focused: Query<Entity, With<Focus>>,
	pointers: Query<Entity, With<PrimaryPointer>>,
	mut commands: Commands,
) {
	let entered = keys
		.read()
		.any(|key| key.state == ButtonState::Pressed && key.logical_key == Key::Enter);
	if !entered {
		return;
	}
	let Ok(target) = focused.single() else { return };
	// the primary pointer carries the event; a placeholder is fine when none is
	// spawned (the consumers read the target, not the pointer).
	let pointer = pointers.iter().next().unwrap_or(Entity::PLACEHOLDER);
	commands.entity(target).trigger(PointerDown::new(pointer));
	commands.entity(target).trigger(PointerUp::new(pointer));
}

/// Observer: infer [`Focusable`] from a newly-added element's tag.
fn infer_focusable(
	ev: On<Add, Element>,
	elements: Query<&Element>,
	mut commands: Commands,
) {
	if let Ok(element) = elements.get(ev.entity) {
		if FOCUSABLE_TAGS.contains(&element.tag()) {
			commands.entity(ev.entity).insert(Focusable);
		}
	}
}

/// Observer: clicking a [`Focusable`] focuses it (the `on_add` hook clears focus
/// from others). `PointerDown` auto-propagates, so a click on text inside a
/// focusable still focuses the focusable.
fn focus_on_click(
	ev: On<PointerDown>,
	focusables: Query<(), With<Focusable>>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if focusables.contains(target) {
		commands.entity(target).insert(Focus);
	}
}

/// System: move [`Focus`] to the next/previous [`Focusable`] in document order on
/// `Tab`/`Shift+Tab`, wrapping at the ends.
///
/// Tab moves focus rather than typing a tab character, so a text field never
/// receives `\t`. Document order is tree pre-order from the roots; focusables
/// with no tree position (eg standalone) trail in entity order so the ring is
/// still stable.
fn tab_focus(
	mut keys: MessageReader<KeyboardInput>,
	focusables: Query<Entity, With<Focusable>>,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
	focused: Query<Entity, With<Focus>>,
	mut commands: Commands,
) {
	// scan this frame's keys for Tab and whether Shift is held. The terminal
	// bridge emits Shift+Tab as a ShiftLeft press bracketing the Tab press, so
	// both land in the same frame's stream.
	let mut tabs = 0i32;
	let mut shift = false;
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		match &key.logical_key {
			Key::Tab => tabs += 1,
			Key::Shift => shift = true,
			_ => {}
		}
	}
	if tabs == 0 {
		return;
	}
	let direction = if shift { -tabs } else { tabs };

	let order = focusables_in_order(&focusables, &children, &parents);
	if order.is_empty() {
		return;
	}

	let current = focused.iter().next();
	let next = match current.and_then(|c| order.iter().position(|&e| e == c)) {
		// wrap forward/back around the focusable ring
		Some(idx) => {
			let len = order.len() as i32;
			((idx as i32 + direction).rem_euclid(len)) as usize
		}
		// nothing focused yet: start at the first
		None => 0,
	};
	commands.entity(order[next]).insert(Focus);
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

/// System: keep the focused entity's [`ElementStateMap`] carrying
/// [`Focused`](ElementState::Focused), so `:focus`/`:focus-visible` rules apply,
/// and clear it from unfocused elements.
///
/// Touches a map only when its `Focused` membership actually changes, so the
/// style cascade re-resolves on focus change but not every frame.
fn sync_focus_state(
	focused: Query<Entity, With<Focus>>,
	mut states: Query<(Entity, &mut ElementStateMap)>,
	without_map: Query<Entity, (With<Focus>, Without<ElementStateMap>)>,
	mut commands: Commands,
) {
	let focus = focused.iter().next();
	for (entity, mut map) in states.iter_mut() {
		let should = Some(entity) == focus;
		if should && !map.contains(&ElementState::Focused) {
			map.insert(ElementState::Focused);
		} else if !should && map.contains(&ElementState::Focused) {
			map.remove(&ElementState::Focused);
		}
	}
	// the focused entity may not have a state map yet; give it one.
	for entity in without_map.iter() {
		commands
			.entity(entity)
			.insert(ElementStateMap::with(ElementState::Focused));
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

	// ── Focusable model, click-to-focus, Tab traversal (Task 12) ──

	/// Whether `entity` currently holds [`Focus`].
	fn is_focused(app: &App, entity: Entity) -> bool {
		app.world().entity(entity).contains::<Focus>()
	}

	/// `<input>`/`<button>`/`<a>` infer [`Focusable`]; a `<span>` does not.
	#[beet_core::test]
	fn infers_focusable_from_tag() {
		let mut app = app();
		let input = app.world_mut().spawn(Element::new("input")).id();
		let span = app.world_mut().spawn(Element::new("span")).id();
		app.update();
		app.world().entity(input).contains::<Focusable>().xpect_true();
		app.world().entity(span).contains::<Focusable>().xpect_false();
	}

	/// Clicking a focusable focuses it.
	#[beet_core::test]
	fn click_focuses() {
		let mut app = app();
		let input = app.world_mut().spawn((Element::new("input"),)).id();
		app.update(); // infer Focusable
		let pointer = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(input)
			.trigger(PointerDown::new(pointer));
		app.update();
		is_focused(&app, input).xpect_true();
	}

	/// `Tab` cycles focus forward through focusables in document order, `Shift+Tab`
	/// back, wrapping at the ends.
	#[beet_core::test]
	fn tab_cycles_focus() {
		let mut app = app();
		// focusables in a tree so document order is the children order (not entity
		// id order), matching a real page.
		let first = app.world_mut().spawn(Focusable).id();
		let second = app.world_mut().spawn(Focusable).id();
		app.world_mut().spawn_empty().add_children(&[first, second]);
		app.update();

		// no focus yet: Tab focuses the first
		type_keys(&mut app, [Key::Tab]);
		is_focused(&app, first).xpect_true();
		// Tab again: second
		type_keys(&mut app, [Key::Tab]);
		is_focused(&app, second).xpect_true();
		// Tab wraps back to first
		type_keys(&mut app, [Key::Tab]);
		is_focused(&app, first).xpect_true();
		// Shift+Tab goes back, wrapping to the last
		type_keys(&mut app, [Key::Shift, Key::Tab]);
		is_focused(&app, second).xpect_true();
	}

	/// A `:focus` rule resolves on the focused element (the `:focus-visible` style
	/// hook), changing its resolved style; clearing focus reverts it.
	#[beet_core::test]
	fn focus_visible_style_applies() {
		use crate::prelude::*;
		use crate::style::*;
		let mut app = App::new();
		// RealtimeParsePlugin runs the style cascade (PostParseTree) each frame.
		app.add_plugins((
			MinimalPlugins,
			InputPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			FocusPlugin,
		));
		let ring = Color::srgb(0.1, 0.4, 0.9);
		app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(vec![
			// `input:focus { border-color: <ring> }`, the focus-visible ring.
			Rule::new()
				.with_selector(Selector::AllOf(vec![
					Selector::tag("input"),
					Selector::state(ElementState::Focused),
				]))
				.with_value(common_props::BorderColorProp, ring),
		]);
		let input = app.world_mut().spawn(Element::new("input")).id();
		// settle: resolve the unfocused style (the focus ring rule does not apply)
		app.update();
		app.update();
		let border_top = |app: &App| {
			app.world()
				.get::<BoxStyle>(input)
				.and_then(|box_style| box_style.border_top)
		};
		let unfocused = border_top(&app);
		// focus it: a couple of frames let the Focused state propagate into the
		// cascade, so the `:focus` rule resolves and recolors the border.
		app.world_mut().entity_mut(input).insert(Focus);
		app.update();
		app.update();
		let focused = border_top(&app);
		focused.xpect_eq(Some(ring));
		(focused != unfocused).xpect_true();
	}

	/// The focused entity carries the `Focused` state (for `:focus`), and it
	/// clears when focus moves away.
	#[beet_core::test]
	fn focus_sets_focused_state() {
		let mut app = app();
		let a = app.world_mut().spawn((Focusable, Focus)).id();
		let b = app.world_mut().spawn(Focusable).id();
		app.update();
		app.world()
			.entity(a)
			.get::<ElementStateMap>()
			.is_some_and(|map| map.contains(&ElementState::Focused))
			.xpect_true();
		// move focus to b; a's Focused state clears, b's sets
		app.world_mut().entity_mut(b).insert(Focus);
		app.update();
		app.world()
			.entity(a)
			.get::<ElementStateMap>()
			.is_some_and(|map| map.contains(&ElementState::Focused))
			.xpect_false();
		app.world()
			.entity(b)
			.get::<ElementStateMap>()
			.is_some_and(|map| map.contains(&ElementState::Focused))
			.xpect_true();
	}
}

