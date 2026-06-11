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
use crate::prelude::*;
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
			{Attribute::bundle_option("name", name)}
			{Attribute::bundle_option("placeholder", placeholder)}
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
			{Attribute::bundle_option("name", name)}
			{Attribute::bundle_option("placeholder", placeholder)}
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
		<select {Classes::new([classes::SELECT, class])} {field} {Attribute::bundle_option("name", name)}>
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
		<form {field} {Attribute::bundle_option("name", name)}>
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
		let mut world = ui_world();
		let root = world.spawn_template(template).unwrap().id();
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

	/// A focused `TextField` widget bound to a document field is editable: typing
	/// updates its `Value`, and the document sync chain carries the edit back into
	/// the field. This is the widget-as-template form of the old `TuiTextBox`,
	/// now actually editable.
	#[cfg(feature = "terminal")]
	#[beet_core::test]
	fn input_widget_edits_bound_document_field() {
		use bevy::input::ButtonState;
		use bevy::input::keyboard::Key;
		use bevy::input::keyboard::KeyCode;
		use bevy::input::keyboard::KeyboardInput;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			DocumentPlugin,
			FocusPlugin,
		));
		// a document with a `name` field, and a TextField bound to it.
		let root = app
			.world_mut()
			.spawn_template(rsx! {
				<div>
					<TextField field={FieldRef::new("name")}/>
				</div>
			})
			.unwrap()
			.id();
		app.world_mut()
			.entity_mut(root)
			.insert(Document::new(val!({ "name": "" })));
		app.update();

		// focus the input (the <input> element) and type "hi".
		let input = app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(app.world())
			.find(|(_, element)| element.tag() == "input")
			.map(|(entity, _)| entity)
			.unwrap();
		app.world_mut().entity_mut(input).insert(Focus);
		for ch in ["h", "i"] {
			app.world_mut().write_message(KeyboardInput {
				key_code: KeyCode::KeyH,
				logical_key: Key::Character(ch.into()),
				state: ButtonState::Pressed,
				text: Some(ch.into()),
				repeat: false,
				window: Entity::PLACEHOLDER,
			});
		}
		// a few frames for the edit to flow through the document sync chain.
		for _ in 0..3 {
			app.update();
		}
		// the document's `name` field now holds the typed text.
		let doc = app.world().get::<Document>(root).unwrap();
		doc.get_field::<String>(&[FieldSegment::key("name")])
			.unwrap()
			.xpect_eq("hi".to_string());
	}
}
