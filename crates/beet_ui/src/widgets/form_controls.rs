//! Form-control widgets: `TextField`, `TextArea`, `Select`, `Form`.
//!
//! Values bind through the [`document`](crate::document) module — an input
//! attached to a [`FieldRef`] reads/writes its value via the resolved
//! [`Document`] entity, regardless of target. This replaces the legacy
//! `FormData → DynamicStruct` web-only path.
//!
//! Variants are mapped one-to-one onto a class name (e.g. `Filled →
//! [`classes::INPUT_FILLED`]). The active rule set (Material Design 3 today)
//! styles these classes via [`RuleSet`]; widget files never hand-roll CSS.
use crate::prelude::FieldRef;
use crate::token::ClassName;
use crate::token::classes;
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
	pub fn class(&self) -> ClassName {
		match self {
			TextFieldVariant::Outlined => classes::INPUT_OUTLINED,
			TextFieldVariant::Filled => classes::INPUT_FILLED,
			TextFieldVariant::Text => classes::INPUT_TEXT,
		}
	}
}

// impl SceneComponent

/// A styled `<input>` text field. Optionally binds to a document field via
/// `field`; when set, the [`FieldRef`] component attaches to the input and it
/// syncs with the resolved [`Document`](crate::document::Document).
#[scene]
pub fn TextField(
	variant: TextFieldVariant,
	name: String,
	placeholder: String,
	field: Option<FieldRef>,
) -> impl Scene {
	let class = variant.class();
	rsx! {
		<input
			{Classes::new([classes::INPUT, class])}
			{field}
			type="text"
			name={name}
			placeholder={placeholder}
		/>
	}
}

/// A styled `<textarea>`. Same variant set and optional `field` binding as
/// [`TextField`].
#[scene]
pub fn TextArea(
	variant: TextFieldVariant,
	name: String,
	placeholder: String,
	field: Option<FieldRef>,
) -> impl Scene {
	let class = variant.class();
	rsx! {
		<textarea
			{Classes::new([classes::INPUT, class])}
			{field}
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
	pub fn class(&self) -> ClassName {
		match self {
			SelectVariant::Outlined => classes::SELECT_OUTLINED,
			SelectVariant::Filled => classes::SELECT_FILLED,
			SelectVariant::Text => classes::SELECT_TEXT,
		}
	}
}

/// A styled `<select>` element. The options are supplied via the default
/// slot (typically `<option>` children). Optionally binds to a document field
/// via `field`.
#[scene]
pub fn Select(
	variant: SelectVariant,
	name: String,
	field: Option<FieldRef>,
) -> impl Scene {
	let class = variant.class();
	rsx! {
		<select {Classes::new([classes::SELECT, class])} {field} name={name}>
			<slot/>
		</select>
	}
}

/// A `<form>` element. Inputs inside the form bind to the form's parent
/// [`Document`](crate::document::Document) via [`FieldRef`]; the optional
/// `field` prop attaches a [`FieldRef`] to the form itself (eg the document
/// root the nested inputs resolve against). The legacy WASM
/// `FormData → DynamicStruct` path is gone.
#[scene]
pub fn Form(name: String, field: Option<FieldRef>) -> impl Scene {
	rsx! {
		<form {field} name={name}>
			<slot/>
		</form>
	}
}
