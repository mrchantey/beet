//! Form-control widgets: `TextField`, `TextArea`, `NumberField`, `Select`,
//! `Form` (and the `Checkbox` next door, which carries its own toggle systems).
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
/// omitted rather than rendered empty. `sensitive` renders a masked
/// `type="password"`, the widget side of [`StringSchema::sensitive`].
#[template]
pub fn TextField(
	variant: TextFieldVariant,
	name: Option<String>,
	placeholder: Option<String>,
	field: Option<FieldRef>,
	#[prop] sensitive: bool,
) -> impl Bundle {
	let class = variant.class();
	let input_type = if sensitive { "password" } else { "text" };
	rsx! {
		<input
			{Classes::new([classes::INPUT, class])}
			{field}
			type={input_type}
			{Attribute::bundle_option("name", name)}
			{Attribute::bundle_option("placeholder", placeholder)}
		/>
	}
}

/// A styled numeric `<input type="number">`, the widget for an integer or float
/// schema. `min`/`max`/`step` mirror the schema's numeric constraints and are
/// omitted when unbounded.
///
/// Typing preserves the bound value's numeric kind: [`Value::edit_text`]
/// stringifies, edits and re-parses into the same variant, so an `i64` field
/// never degrades into a string.
#[template]
pub fn NumberField(
	variant: TextFieldVariant,
	name: Option<String>,
	field: Option<FieldRef>,
	min: Option<f64>,
	max: Option<f64>,
	step: Option<f64>,
) -> impl Bundle {
	let class = variant.class();
	let number = |value: Option<f64>| value.map(|value| value.to_string());
	rsx! {
		<input
			{Classes::new([classes::INPUT, class])}
			{field}
			type="number"
			{Attribute::bundle_option("name", name)}
			{Attribute::bundle_option("min", number(min))}
			{Attribute::bundle_option("max", number(max))}
			{Attribute::bundle_option("step", number(step))}
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
			.add_observer(fire_form_submit)
			.add_observer(super::checkbox::toggle_checkbox_on_activate)
			.add_systems(Update, super::checkbox::sync_checkbox_checked)
			// a `Submit` handler doing real work is async by nature (the
			// `SchemaEditor`'s commit evolves data through a js seam), so the
			// plugin that fires the event declares the bridge that carries it.
			.init_plugin::<AsyncPlugin>();
		// Enter-to-submit needs the keyboard/focus stack, gated with the renderer.
		// `InputPlugin` is a hard dependency of it, not an ambient assumption: the
		// `MessageReader` fails validation every frame without the message type,
		// so the plugin that reads it declares it.
		// Order after `write_focus_input` so a batch of input delivered in one
		// frame (a paste, fast typing, or a cooked-mode terminal sending the whole
		// line at once) has its typed chars committed to the field `Value` before
		// the Enter in that same batch gathers and submits them.
		#[cfg(feature = "tui")]
		app.init_plugin::<bevy::input::InputPlugin>().add_systems(
			Update,
			submit_form_on_enter.after(crate::prelude::write_focus_input),
		);
	}
}

/// Give each form control a default editable [`Value`] so typing lands on it.
///
/// A control bound by `name` alone (not a [`FieldRef`]/[`Document`]) has no
/// `Value` for [`write_focus_input`](crate::prelude::write_focus_input) to edit
/// without this. A [`Checkbox`] never reaches here: it requires its own
/// `Value::Bool(false)`, since its resting state is a boolean, not text.
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
	trigger_form_submit(form, &elements, &values, &mut commands);
}

/// On Enter while a text control inside a form is focused, fire [`Submit`] on the
/// form, the native analogue of a browser submitting a single-field form on
/// Enter. The focused control is resolved per surface, so a multi-tenant terminal
/// submits only the session that pressed Enter. A focused `<button>` is left to
/// the click/activation path ([`fire_form_submit`]).
#[cfg(feature = "tui")]
fn submit_form_on_enter(
	mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
	focused: Query<Entity, With<Focus>>,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
	surfaces: SurfaceQuery,
	values: Query<&Value>,
	mut commands: Commands,
) {
	use bevy::input::ButtonState;
	use bevy::input::keyboard::Key;
	// the surfaces (windows) Enter was pressed on this frame.
	let enter_windows = keys
		.read()
		.filter(|key| {
			key.state == ButtonState::Pressed && key.logical_key == Key::Enter
		})
		.map(|key| key.window)
		.collect::<HashSet<_>>();
	if enter_windows.is_empty() {
		return;
	}
	for target in focused.iter() {
		// only text controls submit on Enter; a select/button/checkbox does not
		// (Enter activates those through `activate_focused_on_enter`)
		if !elements
			.get(target)
			.map(|view| {
				matches!(view.tag(), "input" | "textarea")
					&& view.attribute_string("type") != "checkbox"
			})
			.unwrap_or(false)
		{
			continue;
		}
		if enter_windows
			.iter()
			.any(|window| surfaces.matches(target, *window))
			&& let Some(form) = ancestor_form(&elements, &parents, target)
		{
			trigger_form_submit(form, &elements, &values, &mut commands);
		}
	}
}

/// Gather a form's named controls in document order and fire [`Submit`] on it,
/// each control carrying its typed [`Value`] (an untouched `<select>` falls back
/// to its first option, like a browser). Shared by the button and Enter paths.
fn trigger_form_submit(
	form: Entity,
	elements: &ElementQuery,
	values: &Query<&Value>,
	commands: &mut Commands,
) {
	let gathered = elements
		.iter_descendants_inclusive(form)
		.filter(|view| matches!(view.tag(), "input" | "textarea" | "select"))
		.filter_map(|view| {
			let name = view.attribute("name")?.value.as_str().ok()?.into();
			(name, field_value(elements, values, &view)).xsome()
		})
		.collect::<Map>()
		.xmap(Value::Map);
	commands.trigger(Submit {
		form,
		values: gathered,
	});
}

/// The current value of a form field: its edited [`Value`], or for an untouched
/// `<select>` its first `<option>`'s value (the browser's default selection).
///
/// The control's own [`Value`] passes through untouched rather than being
/// stringified, so a [`Checkbox`] submits a `Bool` and a [`NumberField`] an
/// `Int`/`Float` — [`Submit::values`] is a typed map, which is what lets a
/// schema-driven form's submission validate against that schema.
fn field_value(
	elements: &ElementQuery,
	values: &Query<&Value>,
	view: &ElementView,
) -> Value {
	let edited = values.get(view.entity).cloned().unwrap_or_default();
	let untouched_select = view.tag() == "select"
		&& edited.as_str().is_ok_and(|edited| edited.is_empty());
	if !untouched_select {
		return edited;
	}
	elements
		.iter_descendants_inclusive(view.entity)
		.find(|child| child.tag() == "option")
		.map(|option| option_value(&option))
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
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	// A literal attribute (`type`) and multiple block attributes
	// (`optional_attr` for `name`/`placeholder`) must all survive: each `attr`
	// adds a related attribute entity rather than clobbering the set.
	#[beet_core::test]
	fn text_field_keeps_all_attributes() {
		test_ext::render_html(rsx! {
			<TextField name="email" placeholder="ada@example.com"/>
		})
		.xpect_contains("type=\"text\"")
		.xpect_contains("name=\"email\"")
		.xpect_contains("placeholder=\"ada@example.com\"");
	}

	#[beet_core::test]
	fn text_area_keeps_all_attributes() {
		test_ext::render_html(
			rsx! { <TextArea name="message" placeholder="hi"/> },
		)
		.xpect_contains("name=\"message\"")
		.xpect_contains("placeholder=\"hi\"");
	}

	/// The numeric control carries only the bounds it is given, so an
	/// unconstrained number renders no `min`/`max`/`step` at all.
	#[beet_core::test]
	fn number_field_omits_absent_bounds() {
		test_ext::render_html(rsx! { <NumberField name="count" min=0.0/> })
			.xpect_contains("type=\"number\"")
			.xpect_contains("min=\"0\"")
			.xnot()
			.xpect_contains("max=");
	}

	/// A focused `TextField` widget bound to a document field is editable: typing
	/// updates its `Value`, and the document sync chain carries the edit back into
	/// the field. This is the widget-as-template form of the old `TuiTextBox`,
	/// now actually editable.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn input_widget_edits_bound_document_field() {
		let mut app = test_ext::form_app();
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
			.insert(Document::new(value!({ "name": "" })));
		app.update();

		let (window, _) = test_ext::focus_element(&mut app, "input");
		test_ext::type_text(&mut app, window, "hi");
		// the document's `name` field now holds the typed text.
		app.world()
			.get::<Document>(root)
			.unwrap()
			.get_field::<String>(&[FieldSegment::key("name")])
			.unwrap()
			.xpect_eq("hi".to_string());
	}

	/// Activating a form's submit button fires [`Submit`] on the `<form>`,
	/// carrying its named controls as a [`Value`] map: the typed `name`, and an
	/// untouched `<select>` falling back to its first `<option>`'s value.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn submit_fires_with_field_values() {
		let mut app = test_ext::form_app();
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

		// focus the input and type a name.
		let (window, _) = test_ext::focus_element(&mut app, "input");
		test_ext::type_text(&mut app, window, "Ada");

		// click Submit, firing Submit on the form.
		let button = test_ext::element(&mut app, "button");
		test_ext::click(&mut app, button);

		let values = captured.get().unwrap();
		values
			.get("name")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("Ada");
		// the untouched select defaults to its first option's value.
		values
			.get("role")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("engineer");
	}

	/// Pressing Enter while a text field is focused fires [`Submit`] on its form,
	/// like a browser submitting a single-field form on Enter, so a terminal chat
	/// composer needs no submit button.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn submit_fires_on_enter_in_input() {
		let mut app = test_ext::form_app();
		let captured = Store::new(None::<Value>);
		app.world_mut().add_observer(move |ev: On<Submit>| {
			captured.set(Some(ev.values.clone()));
		});
		app.world_mut()
			.spawn_template(rsx! {
				<Form name="demo">
					<TextField name="message"/>
				</Form>
			})
			.unwrap();
		app.update();

		// focus the input on a surface and type a message.
		let (window, _) = test_ext::focus_element(&mut app, "input");
		test_ext::type_text(&mut app, window, "hi");

		// press Enter on the focused field: Submit fires with the typed value.
		test_ext::press_enter(&mut app, window);

		captured
			.get()
			.unwrap()
			.get("message")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("hi");
	}

	/// Enter on a focused checkbox toggles it rather than submitting: activation
	/// is Enter's job for every non-text control.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn enter_on_a_checkbox_does_not_submit() {
		let mut app = test_ext::form_app();
		let captured = Store::new(None::<Value>);
		app.world_mut().add_observer(move |ev: On<Submit>| {
			captured.set(Some(ev.values.clone()));
		});
		app.world_mut()
			.spawn_template(rsx! {
				<Form><Checkbox name="done"/></Form>
			})
			.unwrap();
		app.update();

		let (window, input) = test_ext::focus_element(&mut app, "input");
		test_ext::press_enter(&mut app, window);
		captured.get().is_none().xpect_true();
		// it toggled instead, and a later submit carries the `Bool`
		app.world()
			.get::<Value>(input)
			.unwrap()
			.xpect_eq(Value::Bool(true));
	}
}

/// Provenance conformance: the properties the schema-driven form and view layers
/// are built on, asserted against the controls they will spawn.
///
/// Provenance here is *structural*, not evented. Every editing widget binds
/// exactly one `(document, field path)` through a [`FieldRef`] and writes only
/// its own local [`Value`]; the bidirectional syncs carry the
/// `(target, field path, new value)` triple, so nothing holds a copy of a
/// document and no edit is ever reconstructed by diffing.
#[cfg(test)]
mod conformance {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Build `template` under a root carrying `document`, settled.
	fn build(
		document: Value,
		template: impl bevy::ecs::template::Template<Output = ()>,
	) -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let root = world.spawn_template(template).unwrap().id();
		world.entity_mut(root).insert(Document::new(document));
		world.update_local();
		(world, root)
	}

	/// A form with one control per leaf, the shape a schema-driven form emits.
	fn form() -> impl bevy::ecs::template::Template<Output = ()> {
		rsx! {
			<Form>
				<TextField field={FieldRef::new("label")}/>
				<TextArea field={FieldRef::new("notes")}/>
				<Select field={FieldRef::new("role")}>
					<option value="engineer">"Engineer"</option>
				</Select>
			</Form>
		}
	}

	/// The bound path and element tag of every [`FieldRef`] in the world, sorted
	/// by path (a query iterates archetypes, not document order).
	fn bindings(world: &mut World) -> Vec<(FieldPath, String)> {
		world
			.query_once::<(&Element, &FieldRef)>()
			.into_iter()
			.map(|(element, field)| {
				(field.field_path.clone(), element.tag().to_string())
			})
			.collect::<Vec<_>>()
			.xtap(|bindings| bindings.sort())
	}

	/// The entity of the first element with `tag`.
	fn element(world: &mut World, tag: &str) -> Entity {
		world
			.query_once::<(Entity, &Element)>()
			.into_iter()
			.find(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// The local [`Value`] of the first element with `tag`.
	fn value_of(world: &mut World, tag: &str) -> Value {
		let entity = element(world, tag);
		world.entity(entity).get::<Value>().unwrap().clone()
	}

	/// Overwrite a control's local [`Value`], the only thing a widget writes.
	fn edit(world: &mut World, tag: &str, value: Value) {
		let entity = element(world, tag);
		*world.entity_mut(entity).get_mut::<Value>().unwrap() = value;
		world.update_local();
	}

	/// The document on `entity`.
	fn document(world: &mut World, entity: Entity) -> Value {
		world.entity(entity).get::<Document>().unwrap().0.clone()
	}

	/// One field entity per editable leaf, the binding contract a schema-driven
	/// form walks a schema to produce: the ref sits on the control itself, one per
	/// control, and never on a wrapper or an `<option>` child.
	#[beet_core::test]
	fn each_control_binds_exactly_one_leaf() {
		let (mut world, _) = build(
			value!({ "label": "buy milk", "notes": "", "role": "engineer" }),
			form(),
		);
		bindings(&mut world).xpect_eq(vec![
			(FieldPath::new(["label"]), "input".to_string()),
			(FieldPath::new(["notes"]), "textarea".to_string()),
			(FieldPath::new(["role"]), "select".to_string()),
		]);
	}

	/// No widget holds a copy of a document: the authored root is the only entity
	/// carrying one, and each control reads through its own local [`Value`].
	#[beet_core::test]
	fn no_widget_holds_a_document() {
		let (mut world, root) = build(
			value!({ "label": "buy milk", "notes": "", "role": "engineer" }),
			form(),
		);
		world
			.query_once::<(Entity, &Document)>()
			.into_iter()
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>()
			.xpect_eq(vec![root]);
		value_of(&mut world, "input").xpect_eq(Value::Str("buy milk".into()));
	}

	/// A widget's only write is its local [`Value`]; the sync carries it to the
	/// bound field and leaves every sibling leaf alone, which is what lets a
	/// schema-driven form spawn one control per leaf and own no state itself.
	#[beet_core::test]
	fn an_edit_reaches_only_its_own_leaf() {
		let (mut world, root) = build(
			value!({
				"label": "buy milk",
				"notes": "later",
				"role": "engineer"
			}),
			form(),
		);
		edit(&mut world, "input", Value::Str("buy bread".into()));

		document(&mut world, root).xpect_eq(value!({
			"label": "buy bread",
			"notes": "later",
			"role": "engineer"
		}));
	}

	/// Clearing a control reaches the document: an emptied widget writes `null`
	/// rather than silently leaving the old value in place.
	#[beet_core::test]
	fn clearing_a_control_reaches_the_document() {
		let (mut world, root) = build(
			value!({ "label": "buy milk", "notes": "", "role": "engineer" }),
			form(),
		);
		edit(&mut world, "input", Value::Null);

		document(&mut world, root).xpect_eq(value!({
			"label": null,
			"notes": "",
			"role": "engineer"
		}));
	}

	/// A [`DocRef`] retargets a whole form at a document it is not nested under,
	/// which is how an editor form binds the document it edits from anywhere in
	/// the tree. The host document it sits inside is never touched.
	#[beet_core::test]
	fn a_doc_ref_form_edits_the_targeted_document() {
		let mut world = world_ext::ui_world();
		let foreign = world
			.spawn(Document::new(value!({ "label": "foreign" })))
			.id();
		let root = world
			.spawn_template(rsx! {
				<Form {DocRef(foreign)}>
					<TextField field={FieldRef::new("label")}/>
				</Form>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(value!({ "label": "host" })));
		world.update_local();

		// the control read the targeted document, not the one it is nested under
		value_of(&mut world, "input").xpect_eq(Value::Str("foreign".into()));

		edit(&mut world, "input", Value::Str("edited".into()));
		document(&mut world, foreign).xpect_eq(value!({ "label": "edited" }));
		document(&mut world, root).xpect_eq(value!({ "label": "host" }));
	}
}
