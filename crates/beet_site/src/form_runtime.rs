//! The non-web form runtime: makes the `docs/design/form` demo interactive in
//! the terminal, the native counterpart of its web `<script>` (which the
//! terminal skips).
//!
//! On a form submit (a `<button>` activation inside a `<form>`, fired as
//! `PointerUp` by the focus/click path), it gathers the form's named fields and
//! writes them as pretty JSON into the `#form-output` element, mirroring the web
//! `JSON.stringify(FormData, null, 2)` behaviour.

use beet::prelude::*;

/// Registers the native form runtime: editable fields plus submit-to-JSON.
pub struct FormRuntimePlugin;

impl Plugin for FormRuntimePlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(ensure_form_field_value)
			.add_observer(on_form_submit);
	}
}

/// Give each form control a default editable [`Value`] so typing lands on it.
///
/// The demo's fields bind by `name` (not a [`FieldRef`]/[`Document`]), so without
/// this they would have no `Value` for [`write_focus_input`] to edit.
fn ensure_form_field_value(
	ev: On<Add, Element>,
	elements: Query<&Element>,
	has_value: Query<(), With<Value>>,
	mut commands: Commands,
) {
	let Ok(element) = elements.get(ev.entity) else {
		return;
	};
	if matches!(element.tag(), "input" | "textarea" | "select")
		&& !has_value.contains(ev.entity)
	{
		commands.entity(ev.entity).insert(Value::str(""));
	}
}

/// On a button activation inside a form, gather the named fields and write their
/// pretty JSON into the `#form-output` element.
fn on_form_submit(
	ev: On<PointerUp>,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	values: Query<&Value>,
	mut commands: Commands,
) {
	// `PointerUp` propagates up the tree, firing this global observer per ancestor;
	// act exactly once, at the activated `<button>` itself.
	let target = ev.event_target();
	let is_button =
		elements.get(target).map(|view| view.tag() == "button").unwrap_or(false);
	if !is_button {
		return;
	}
	let Some(form) = ancestor_form(&elements, &parents, target) else {
		return;
	};

	// the named fields in document order, each field's typed `Value` (a select
	// with no edit falls back to its first option, like a browser).
	let fields: Vec<(String, String)> = elements
		.iter_descendants_inclusive(form)
		.filter(|view| matches!(view.tag(), "input" | "textarea" | "select"))
		.filter_map(|view| {
			let name =
				view.attribute("name")?.value.as_str().ok()?.to_string();
			let value = field_value(&elements, &values, &view);
			Some((name, value))
		})
		.collect();
	let json = pretty_json(&fields);

	// replace the `#form-output` placeholder text with the JSON.
	if let Some(output) = elements
		.iter()
		.find(|view| view.attribute_string("id") == "form-output")
	{
		if let Some((text, _)) = output.inner_text {
			commands.entity(text).insert(Value::str(json));
		}
	}
}

/// The current value of a form field: its edited [`Value`], or for an untouched
/// `<select>` its first `<option>`'s `value` (the browser's default selection).
fn field_value(
	elements: &ElementQuery,
	values: &Query<&Value>,
	view: &ElementView,
) -> String {
	let edited = values
		.get(view.entity)
		.ok()
		.and_then(|value| value.as_str().ok())
		.map(str::to_string)
		.unwrap_or_default();
	if !edited.is_empty() || view.tag() != "select" {
		return edited;
	}
	elements
		.iter_descendants_inclusive(view.entity)
		.find(|child| child.tag() == "option")
		.map(|option| option.attribute_string("value"))
		.unwrap_or_default()
}

/// The nearest `<form>` ancestor of `start` (inclusive), if any.
fn ancestor_form(
	elements: &ElementQuery,
	parents: &Query<&ChildOf>,
	start: Entity,
) -> Option<Entity> {
	let mut current = Some(start);
	while let Some(entity) = current {
		if elements.get(entity).map(|view| view.tag() == "form").unwrap_or(false)
		{
			return Some(entity);
		}
		current = parents.get(entity).ok().map(|child_of| child_of.parent());
	}
	None
}

/// Pretty-print the `(name, value)` fields as a 2-space-indented JSON object,
/// matching the web `JSON.stringify(data, null, 2)`.
fn pretty_json(fields: &[(String, String)]) -> String {
	if fields.is_empty() {
		return "{}".to_string();
	}
	let body = fields
		.iter()
		.map(|(key, value)| {
			format!("  \"{}\": \"{}\"", escape(key), escape(value))
		})
		.collect::<Vec<_>>()
		.join(",\n");
	format!("{{\n{body}\n}}")
}

/// Minimal JSON string escaping for the field keys and values.
fn escape(text: &str) -> String {
	text.replace('\\', "\\\\")
		.replace('"', "\\\"")
		.replace('\n', "\\n")
}
