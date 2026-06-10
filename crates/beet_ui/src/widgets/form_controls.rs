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
use crate::token::ClassName;
use crate::style::material::classes;
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
/// syncs with the resolved [`Document`](beet_core::prelude::Document).
///
/// `name` and `placeholder` are optional — when unset their attributes are
/// omitted rather than rendered empty.
#[template]
pub fn TextField(
	variant: TextFieldVariant,
	name: Option<String>,
	placeholder: Option<String>,
	field: Option<FieldRef>,
) -> impl Bundle {
	let class = variant.class();
	rsx! {
		<input
			{Classes::new([classes::INPUT, class])}
			{field}
			type="text"
			{optional_attr("name", name)}
			{optional_attr("placeholder", placeholder)}
		/>
	}
}

/// A styled `<textarea>`. Same variant set and optional `field` binding as
/// [`TextField`]; `name` and `placeholder` are likewise optional.
#[template]
pub fn TextArea(
	variant: TextFieldVariant,
	name: Option<String>,
	placeholder: Option<String>,
	field: Option<FieldRef>,
) -> impl Bundle {
	let class = variant.class();
	rsx! {
		<textarea
			{Classes::new([classes::INPUT, class])}
			{field}
			{optional_attr("name", name)}
			{optional_attr("placeholder", placeholder)}
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
/// via `field`; `name` is omitted when unset.
#[template]
pub fn Select(
	variant: SelectVariant,
	name: Option<String>,
	field: Option<FieldRef>,
) -> impl Bundle {
	let class = variant.class();
	rsx! {
		<select {Classes::new([classes::SELECT, class])} {field} {optional_attr("name", name)}>
			<Slot/>
		</select>
	}
}

/// A `<form>` element. Inputs inside the form bind to the form's parent
/// [`Document`](beet_core::prelude::Document) via [`FieldRef`]; the optional
/// `field` prop attaches a [`FieldRef`] to the form itself (eg the document
/// root the nested inputs resolve against). The legacy WASM
/// `FormData → DynamicStruct` path is gone.
#[template]
pub fn Form(name: Option<String>, field: Option<FieldRef>) -> impl Bundle {
	rsx! {
		<form {field} {optional_attr("name", name)}>
			<Slot/>
		</form>
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Render a template to an HTML string through the substrate.
	fn render_html(template: impl bevy::ecs::template::Template<Output = ()>) -> String {
		let mut world = test_world();
		let root = world.spawn_template(template).id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
	}

	// A literal attribute (`type`) and multiple block attributes
	// (`optional_attr` for `name`/`placeholder`) must all survive: each `attr`
	// adds a related attribute entity rather than clobbering the set.
	#[beet_core::test]
	fn text_field_keeps_all_attributes() {
		render_html(rsx! {
			<TextField name="email" placeholder="ada@example.com"/>
		})
		.xpect_contains("type=\"text\"")
		.xpect_contains("name=\"email\"")
		.xpect_contains("placeholder=\"ada@example.com\"");
	}

	#[beet_core::test]
	fn text_area_keeps_all_attributes() {
		render_html(rsx! { <TextArea name="message" placeholder="hi"/> })
			.xpect_contains("name=\"message\"")
			.xpect_contains("placeholder=\"hi\"");
	}
}
