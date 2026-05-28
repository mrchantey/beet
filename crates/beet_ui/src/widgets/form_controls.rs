//! Form-control widgets: `TextField`, `TextArea`, `Select`, `Form`.
//!
//! Values bind through the [`document`](crate::document) module — an input
//! attached to a [`FieldRef`] reads/writes its value via the resolved
//! [`Document`] entity, regardless of target. This replaces the legacy
//! `FormData → DynamicStruct` web-only path.
//!
//! Variants are mapped one-to-one onto a class name (e.g. `Filled →
//! "input-filled"`). The active rule set (Material Design 3 today) styles
//! these classes via [`RuleSet`]; widget files never hand-roll CSS.
use beet_core::prelude::*;

/// Variant style for a [`TextField`] or [`TextArea`], mapped onto a class
/// (`input-outlined`, `input-filled`, `input-text`).
#[derive(Default, Clone, Reflect)]
pub enum TextFieldVariant {
	#[default]
	Outlined,
	Filled,
	Text,
}

impl TextFieldVariant {
	/// The semantic class name for this variant.
	pub fn class(&self) -> &'static str {
		match self {
			TextFieldVariant::Outlined => "input-outlined",
			TextFieldVariant::Filled => "input-filled",
			TextFieldVariant::Text => "input-text",
		}
	}
}

/// A styled `<input>` text field. Optionally binds to a document field via
/// `field`; if set, the input syncs with the resolved [`Document`].
#[scene]
pub fn TextField(
	variant: TextFieldVariant,
	name: String,
	placeholder: String,
) -> impl Scene {
	let class = variant.class();
	rsx! {
		<input
			{Classes::new(["input", class])}
			type="text"
			name={name}
			placeholder={placeholder}
		/>
	}
}

/// A styled `<textarea>`. Same variant set as [`TextField`].
#[scene]
pub fn TextArea(
	variant: TextFieldVariant,
	name: String,
	placeholder: String,
) -> impl Scene {
	let class = variant.class();
	rsx! {
		<textarea
			{Classes::new(["input", class])}
			name={name}
			placeholder={placeholder}
		/>
	}
}

/// Variant style for a [`Select`].
#[derive(Default, Clone, Reflect)]
pub enum SelectVariant {
	#[default]
	Outlined,
	Filled,
	Text,
}

impl SelectVariant {
	pub fn class(&self) -> &'static str {
		match self {
			SelectVariant::Outlined => "select-outlined",
			SelectVariant::Filled => "select-filled",
			SelectVariant::Text => "select-text",
		}
	}
}

/// A styled `<select>` element. The options are supplied via the default
/// slot (typically `<option>` children).
#[scene]
pub fn Select(variant: SelectVariant, name: String) -> impl Scene {
	let class = variant.class();
	rsx! {
		<select {Classes::new(["select", class])} name={name}>
			<slot/>
		</select>
	}
}

/// A `<form>` element. Inputs inside the form bind to the form's parent
/// [`Document`] via [`FieldRef`]; submitting the form is handled by the
/// document module (the legacy WASM `FormData → DynamicStruct` is gone).
#[scene]
pub fn Form(name: String) -> impl Scene {
	rsx! {
		<form name={name}>
			<slot/>
		</form>
	}
}
