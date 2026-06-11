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
//!
//! [`FormPlugin`] makes a form behave like a browser form: each control is
//! editable by default, and activating the submit button fires a [`Submit`]
//! event on the `<form>` carrying its named fields as a [`Value`] map, the
//! native analogue of a web form's `submit` event + `FormData`.
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

/// Fired on a `<form>` element when its submit button is activated, the native
/// analogue of a web form's `submit` event.
///
/// [`Submit::values`] carries the gathered named controls as a [`Value`] map
/// (`name -> value`), the equivalent of the browser's `FormData`. An untouched
/// `<select>` falls back to its first `<option>`'s value, like a browser.
#[derive(Debug, EntityEvent)]
pub struct Submit {
	/// The `<form>` the submit targets.
	#[event_target]
	pub form: Entity,
	/// The named field values, `name -> value`, as a [`Value::Map`].
	pub values: Value,
}

/// Makes [`Form`] controls behave like a browser form: editable by default, and
/// a submit-button activation fires [`Submit`] on the `<form>`.
///
/// Backend-agnostic: the generic gathering and firing live here; a consumer
/// observes [`Submit`] to do something with the values (eg render them).
#[derive(Default)]
pub struct FormPlugin;

impl Plugin for FormPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(ensure_form_field_value)
			.add_observer(fire_form_submit);
	}
}

/// Give each form control a default editable [`Value`] so typing lands on it.
///
/// A control bound by `name` alone (not a [`FieldRef`]/[`Document`]) has no
/// `Value` for [`write_focus_input`](crate::prelude::write_focus_input) to edit
/// without this.
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

/// On a button activation inside a form, gather the named fields and fire
/// [`Submit`] on the `<form>` carrying their values.
fn fire_form_submit(
	ev: On<PointerUp>,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	values: Query<&Value>,
	mut commands: Commands,
) {
	// `PointerUp` propagates up the tree, firing this global observer per
	// ancestor; act exactly once, at the activated `<button>` itself.
	let target = ev.event_target();
	let is_button = elements
		.get(target)
		.map(|view| view.tag() == "button")
		.unwrap_or(false);
	if !is_button {
		return;
	}
	let Some(form) = ancestor_form(&elements, &parents, target) else {
		return;
	};

	// the named controls in document order, each its typed `Value` (a select
	// with no edit falls back to its first option, like a browser).
	let values = elements
		.iter_descendants_inclusive(form)
		.filter(|view| matches!(view.tag(), "input" | "textarea" | "select"))
		.filter_map(|view| {
			let name = view.attribute("name")?.value.as_str().ok()?.into();
			(name, field_value(&elements, &values, &view)).xsome()
		})
		.collect::<Map>()
		.xmap(Value::Map);
	commands.trigger(Submit { form, values });
}

/// The current value of a form field: its edited [`Value`], or for an untouched
/// `<select>` its first `<option>`'s `value` (the browser's default selection).
fn field_value(
	elements: &ElementQuery,
	values: &Query<&Value>,
	view: &ElementView,
) -> Value {
	let edited = values
		.get(view.entity)
		.ok()
		.and_then(|value| value.as_str().ok())
		.unwrap_or_default();
	if !edited.is_empty() || view.tag() != "select" {
		return Value::str(edited);
	}
	elements
		.iter_descendants_inclusive(view.entity)
		.find(|child| child.tag() == "option")
		.map(|option| option.attribute_string("value"))
		.unwrap_or_default()
		.xmap(Value::str)
}

/// The nearest `<form>` ancestor of `start` (inclusive), if any.
fn ancestor_form(
	elements: &ElementQuery,
	parents: &Query<&ChildOf>,
	start: Entity,
) -> Option<Entity> {
	let mut current = Some(start);
	while let Some(entity) = current {
		if elements
			.get(entity)
			.map(|view| view.tag() == "form")
			.unwrap_or(false)
		{
			return Some(entity);
		}
		current = parents.get(entity).ok().map(|child_of| child_of.parent());
	}
	None
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
	#[cfg(feature = "tui")]
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

	/// Activating a form's submit button fires [`Submit`] on the `<form>`,
	/// carrying its named controls as a [`Value`] map: the typed `name`, and an
	/// untouched `<select>` falling back to its first `<option>`'s value.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn submit_fires_with_field_values() {
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
			FormPlugin,
		));
		// capture the carried values when Submit fires.
		let captured = Store::new(None::<Value>);
		app.world_mut().add_observer(move |ev: On<Submit>| {
			captured.set(Some(ev.values.clone()));
		});
		app.world_mut()
			.spawn_template(rsx! {
				<Form name="demo">
					<TextField name="name"/>
					<Select name="role">
						<option value="engineer">"Engineer"</option>
						<option value="designer">"Designer"</option>
					</Select>
					<Button>"Submit"</Button>
				</Form>
			})
			.unwrap();
		app.update();

		let element = |app: &mut App, tag: &str| {
			app.world_mut()
				.query::<(Entity, &Element)>()
				.iter(app.world())
				.find(|(_, el)| el.tag() == tag)
				.map(|(entity, _)| entity)
				.unwrap()
		};
		// focus the input and type a name.
		let input = element(&mut app, "input");
		app.world_mut().entity_mut(input).insert(Focus);
		for ch in ["A", "d", "a"] {
			app.world_mut().write_message(KeyboardInput {
				key_code: KeyCode::KeyA,
				logical_key: Key::Character(ch.into()),
				state: ButtonState::Pressed,
				text: Some(ch.into()),
				repeat: false,
				window: Entity::PLACEHOLDER,
			});
		}
		app.update();

		// click Submit, firing Submit on the form.
		let button = element(&mut app, "button");
		let pointer = app.world_mut().spawn_empty().id();
		app.world_mut().entity_mut(button).trigger(PointerUp::new(pointer));
		app.update();

		let values = captured.get().unwrap();
		values.get("name").unwrap().as_str().unwrap().xpect_eq("Ada");
		// the untouched select defaults to its first option's value.
		values.get("role").unwrap().as_str().unwrap().xpect_eq("engineer");
	}
}
