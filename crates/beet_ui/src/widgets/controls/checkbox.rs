//! `Checkbox`: the boolean form control, an `<input type="checkbox">` bound to
//! a document field.
//!
//! The only control that produces a [`Value::Bool`]: a text control edits
//! through [`Value::edit_text`], which rejects a boolean outright, so a bool
//! leaf has no other widget to dispatch to.
//!
//! Activation — a click, or Enter while focused via `activate_focused_on_enter`,
//! which synthesizes the same `PointerUp` — flips the local [`Value`]
//! ([`toggle_checkbox_on_activate`], registered by
//! [`FormPlugin`](super::FormPlugin)); the document sync chain carries the
//! flip into the bound field, exactly as typing does for a text field. The
//! value is the box's state, read through [`ElementView::checked`]: HTML
//! writes `checked` from it, the terminal paints a `[x]`/`[ ]` marker and
//! markdown a task list's box.
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
pub fn Checkbox(
	name: Option<String>,
	field: Option<FieldRef>,
	write: Option<WritePolicy>,
) -> impl Bundle {
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
			{write}
			{Attribute::bundle_option("name", name)}
		/>
	}
}

/// Marks a [`Checkbox`]'s `<input>`, the target of
/// [`toggle_checkbox_on_activate`].
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::test_ext;
	use beet_core::prelude::*;

	/// The served `checked` attribute is the bound document value's: present
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
			// settle the doc sync
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
