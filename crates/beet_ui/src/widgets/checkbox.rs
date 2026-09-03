//! `Checkbox`: the boolean form control, an `<input type="checkbox">` bound to
//! a document field.
//!
//! The only control that produces a [`Value::Bool`]: a text control edits
//! through [`Value::edit_text`], which rejects a boolean outright, so a bool
//! leaf has no other widget to dispatch to.
//!
//! Activation — a click, or Enter while focused via `activate_focused_on_enter`,
//! which synthesizes the same `PointerUp` — flips the local [`Value`]
//! ([`toggle_checkbox_on_activate`]); the document sync chain carries the flip
//! into the bound field, exactly as typing does for a text field.
//! [`sync_checkbox_checked`] keeps the bare `checked` attribute mirroring the
//! value so a served render is correct HTML; the terminal paints a `[x]`/`[ ]`
//! marker from the value instead (`decorate::checkbox_marker`). Both are
//! registered by [`FormPlugin`](super::FormPlugin).
use crate::prelude::*;
use beet_core::prelude::*;

/// A boolean `<input type="checkbox">`, optionally bound to a document field.
///
/// A bound checkbox writes `false` into an absent field rather than the
/// [`FieldRef`] default of `null`, so an untouched box round trips as the
/// boolean it is. Labels are the caller's: a checkbox carries none, so
/// [`DynamicForm`](crate::prelude::DynamicForm) wraps every control in the same
/// `<label>` row.
///
/// `<Checkbox field={FieldRef::new("done")}/>`
#[template]
pub fn Checkbox(name: Option<String>, field: Option<FieldRef>) -> impl Bundle {
	// the default policy seeds an absent field null; a checkbox's resting state
	// is `false`. An author-provided policy is kept.
	let field = field.map(|field| match &field.on_missing {
		OnMissing::Default(Value::Null) => field.with_init(false),
		_ => field,
	});
	rsx! {
		<input
			type="checkbox"
			{Classes::new([classes::CHECKBOX])}
			{CheckboxInput}
			{field}
			{Attribute::bundle_option("name", name)}
		/>
	}
}

/// Marks a [`Checkbox`]'s `<input>`, the target of
/// [`toggle_checkbox_on_activate`] and [`sync_checkbox_checked`].
///
/// Requires its own `Value::Bool(false)`: a checkbox's resting state is a
/// boolean, so it never takes the empty-string seed
/// `ensure_form_field_value` gives a text control.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component, Default)]
#[require(Value = Value::Bool(false))]
pub struct CheckboxInput;

/// Observer: activating a checkbox flips its [`Value`]. A non-`Bool` value (an
/// unbound control's seed, a document holding something else) counts as
/// unchecked, so the first activation checks it.
pub(super) fn toggle_checkbox_on_activate(
	ev: On<PointerUp>,
	checkboxes: Query<(), With<CheckboxInput>>,
	mut values: Query<&mut Value>,
) {
	// the event bubbles; act only at the checkbox input itself.
	let target = ev.event_target();
	if !checkboxes.contains(target) {
		return;
	}
	if let Ok(mut value) = values.get_mut(target) {
		let checked = matches!(*value, Value::Bool(true));
		value.set_if_neq(Value::Bool(!checked));
	}
}

/// System: keep a checkbox's bare `checked` attribute present exactly when its
/// [`Value`] is `true`, so a served render emits correct HTML. The terminal
/// marker reads the value directly, so this is markup state only.
pub(super) fn sync_checkbox_checked(
	checkboxes: Populated<
		(Entity, &Value),
		(With<CheckboxInput>, Changed<Value>),
	>,
	attributes: Query<&Attributes>,
	attr_keys: Query<&Attribute>,
	mut commands: Commands,
) {
	for (entity, value) in checkboxes.iter() {
		let checked = matches!(value, Value::Bool(true));
		let attr = attr_entity(&attributes, &attr_keys, entity, "checked");
		match (checked, attr) {
			// a null-valued attribute renders as the bare `checked` form
			(true, None) => {
				commands.spawn((
					AttributeOf::new(entity),
					Attribute::new("checked"),
				));
			}
			(false, Some(attr)) => commands.entity(attr).despawn(),
			_ => {}
		}
	}
}

#[cfg(test)]
mod test {
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The bare `checked` attribute tracks the bound document value: present
	/// when true, absent when false.
	#[beet_core::test]
	fn checked_attribute_mirrors_value() {
		let render = |done: bool| {
			let mut world = test_ext::form_world();
			let root = world
				.spawn_template(rsx! {
					<div><Checkbox field={FieldRef::new("done")}/></div>
				})
				.unwrap()
				.id();
			world
				.entity_mut(root)
				.insert(Document::new(value!({ "done": done })));
			// settle the doc sync then the attribute mirror
			world.update_local();
			world.update_local();
			test_ext::render_world(&mut world, root)
		};
		render(true).xpect_contains("checked");
		render(false).xnot().xpect_contains("checked");
	}

	/// Clicking a checkbox, and pressing Enter while it is focused, both toggle
	/// the bound document field — the only path from a widget to a `Bool`.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn click_and_enter_toggle_bound_field() {
		let mut app = test_ext::form_app();
		let root = app
			.world_mut()
			.spawn_template(rsx! {
				<div><Checkbox field={FieldRef::new("done")}/></div>
			})
			.unwrap()
			.id();
		app.world_mut()
			.entity_mut(root)
			.insert(Document::new(value!({ "done": false })));
		app.update();

		let (window, input) = test_ext::focus_element(&mut app, "input");
		let done = |app: &App| {
			app.world()
				.get::<Document>(root)
				.unwrap()
				.get_field::<bool>(&[FieldSegment::key("done")])
				.unwrap()
		};
		// a click checks it
		test_ext::click(&mut app, input);
		done(&app).xpect_true();
		// Enter on the focused input unchecks it (keyboard activation)
		test_ext::press_enter(&mut app, window);
		done(&app).xpect_false();
	}
}
