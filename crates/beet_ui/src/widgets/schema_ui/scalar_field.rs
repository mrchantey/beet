//! The scalar arms of a [`DynamicForm`](super::DynamicForm): the leaves that
//! bind one value and need no walk of their own.
//!
//! Each is one control from [`controls`](crate::widgets::controls) carrying the
//! leaf's path as its `name`, so a schema's hints and constraints arrive as the
//! widget's own bounds rather than as a second validation pass.
use super::field_layout::labeled;
use super::field_layout::widget;
use crate::prelude::*;
use beet_core::prelude::*;

/// The boolean arm: the [`Checkbox`], the one control that produces a `Bool`.
pub(super) fn bool_field(field: FieldRef, label: Option<String>) -> Snippet {
	labeled(
		label,
		widget(Checkbox {
			name: PropOpt::some(field.field_path.to_string()),
			field: PropOpt::some(field),
		}),
	)
}

/// The string arm: a [`TextArea`] for multiline prose, else a [`TextField`],
/// masked when the schema marks the value sensitive.
pub(super) fn string_field(
	schema: &StringSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let name = PropOpt::some(field.field_path.to_string());
	let widget = match schema.multiline {
		true => widget(TextArea {
			name,
			field: PropOpt::some(field),
			..default()
		}),
		false => widget(TextField {
			name,
			field: PropOpt::some(field),
			sensitive: schema.sensitive,
			..default()
		}),
	};
	labeled(label, widget)
}

/// The numeric arms: a [`NumberField`] carrying whichever `Min`/`Max`/`Step`
/// bounds the schema declares, the last of each kind winning.
///
/// A macro because the three numeric schemas share the shape but not the
/// constraint type.
macro_rules! number_arm {
	($arm:ident, $schema:ident, $constraint:ident) => {
		#[doc = concat!("The `", stringify!($schema), "` arm.")]
		pub(super) fn $arm(
			schema: &$schema,
			field: FieldRef,
			label: Option<String>,
		) -> Snippet {
			let mut bounds: (Option<f64>, Option<f64>, Option<f64>) =
				(None, None, None);
			for constraint in &schema.constraints {
				match constraint {
					$constraint::Min(min) => bounds.0 = Some(min.value as f64),
					$constraint::Max(max) => bounds.1 = Some(max.value as f64),
					$constraint::Step(step) => {
						bounds.2 = Some(step.value as f64)
					}
				}
			}
			number_field(bounds, field, label)
		}
	};
}

number_arm!(i64_field, I64Schema, I64Constraint);
number_arm!(u64_field, U64Schema, U64Constraint);
number_arm!(f64_field, F64Schema, F64Constraint);

/// The [`NumberField`] the three numeric arms share, carrying their extracted
/// `(min, max, step)` as its bounds.
fn number_field(
	(min, max, step): (Option<f64>, Option<f64>, Option<f64>),
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	labeled(
		label,
		widget(NumberField {
			name: PropOpt::some(field.field_path.to_string()),
			field: PropOpt::some(field),
			min: PropOpt(min),
			max: PropOpt(max),
			step: PropOpt(step),
			..default()
		}),
	)
}

#[cfg(test)]
mod test {
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn bool_dispatches_to_a_checkbox() {
		test_ext::form_html(ValueSchema::Bool(default()))
			.xpect_contains("type=\"checkbox\"");
	}

	/// A number carries its constraint bounds onto the control, so the widget
	/// enforces what the schema declares without restating it.
	#[beet_core::test]
	fn numbers_dispatch_to_a_number_field_with_bounds() {
		test_ext::form_html(ValueSchema::I64(I64Schema {
			constraints: vec![
				I64Constraint::Min(I64Min {
					value: 1,
					behavior: default(),
				}),
				I64Constraint::Max(I64Max {
					value: 9,
					behavior: default(),
				}),
				I64Constraint::Step(I64Step {
					value: 2,
					behavior: default(),
				}),
			],
		}))
		.xpect_contains("type=\"number\"")
		.xpect_contains("min=\"1\"")
		.xpect_contains("max=\"9\"")
		.xpect_contains("step=\"2\"");
	}

	#[beet_core::test]
	fn strings_dispatch_by_hint() {
		test_ext::form_html(ValueSchema::String(default()))
			.xpect_contains("type=\"text\"");
		test_ext::form_html(ValueSchema::String(
			StringSchema::default().multiline(),
		))
		.xpect_contains("<textarea");
		test_ext::form_html(ValueSchema::String(
			StringSchema::default().sensitive(),
		))
		.xpect_contains("type=\"password\"");
	}
}
