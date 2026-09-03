//! [`DynamicForm`]: a `<form>` generated from a [`ValueSchema`], one control
//! per editable leaf.
//!
//! The bridge between the schema layer and the form controls. A schema kind
//! picks the widget, its hints and constraints supply the widget's bounds, and
//! a struct schema recurses into a disclosure group whose [`FieldRef`]s extend
//! the parent's path.
//!
//! It owns no state and performs no writes: every leaf binds one
//! `(document, field path)` through its own [`FieldRef`] and writes only its
//! local [`Value`], so the bidirectional syncs carry the edit and nothing here
//! ever holds a copy of a document. Each control also carries the leaf's path
//! as its `name`, so the whole form gathers as a typed [`Value`] map on
//! [`Submit`] — the commit boundary a transactional edit rides.
//!
//! Surface-agnostic: the widgets it spawns are ordinary elements, so the same
//! form paints in a terminal and serves as HTML.
use crate::prelude::*;
use beet_core::prelude::*;

/// Cap on nested [`ValueSchema::Struct`] recursion. A schema graph is finite by
/// construction ([`SchemaResolver`] bounds a reference chain and leaves a cycle
/// unresolved), so this is a defensive bound; a deeper subtree renders as an
/// [`UneditableField`].
const MAX_DEPTH: usize = 8;

/// A `<form>` whose controls are generated from `schema`, each bound to its own
/// leaf of the document `field` points into:
///
/// - `Bool` -> [`Checkbox`]
/// - `I64`/`U64`/`F64` -> [`NumberField`], carrying the schema's `Min`/`Max`/
///   `Step` constraints as bounds
/// - `String` -> [`TextField`], masked when `sensitive`, a [`TextArea`] when
///   `multiline`
/// - an all-unit `Enum` -> [`Select`] with an option per variant
/// - `Optional` -> its inner schema (a missing value reads as empty)
/// - `Reference` -> the schema it names, resolved against the [`SchemaRegistry`]
/// - `Struct` -> a labelled group, one nested control per field, each
///   [`FieldRef`] extending this one's path. At the top level the fields are the
///   form's own rows; nested, they sit in an open `<details>` disclosure.
/// - anything else (`List`/`Map`/`Tuple`/`Entity`/`Bytes`/a payload-carrying
///   `Enum`/`Any`/an unresolved `Reference`) -> a read-only
///   [`UneditableField`], since no control can produce a valid value for it.
///   A list is the [`DynamicView`](crate::prelude::DynamicView)'s side of the
///   contract, and an `Entity` reference wants a picker no phase has built.
///
/// The default slot lands inside the `<form>` after the generated controls, ie
/// where a submit [`Button`] goes.
///
/// ```rsx
/// <DynamicForm schema={ValueSchema::of::<TodoItem>()} field={FieldRef::new("draft")}>
///   <Button>"Add"</Button>
/// </DynamicForm>
/// ```
#[template(system)]
pub fn DynamicForm(
	/// The schema of the value this form edits.
	#[prop(required)]
	schema: ValueSchema,
	/// The document field the form edits: the path every generated control's
	/// own [`FieldRef`] extends. Defaults to the whole document.
	#[prop]
	field: FieldRef,
	/// The by-name registry a [`ValueSchema::Reference`] resolves against;
	/// absent until [`DocumentPlugin`] has initialized it, which defers every
	/// reference to an [`UneditableField`] exactly as an unregistered name does.
	schemas: Option<Res<SchemaRegistry>>,
) -> impl Bundle {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	rsx! {
		<Form>
			{schema_field(resolver, schema, field, None, 0)}
			<Slot/>
		</Form>
	}
}

/// Marks a [`DynamicForm`] leaf whose schema has no editing widget, carrying
/// the schema that found none.
///
/// The leaf still renders, read-only, so a form keeps its shape with only the
/// editing missing — the same bargain an unregistered tag strikes. The gap is
/// announced once, where it is built.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct UneditableField(pub ValueSchema);

/// `(min, max, step)` as `f64` from a numeric schema's constraints, the last of
/// each kind winning. A macro because the three numeric schemas share the shape
/// but not the constraint type.
macro_rules! number_bounds {
	($schema:expr, $constraint:ident) => {{
		let mut bounds: (Option<f64>, Option<f64>, Option<f64>) =
			(None, None, None);
		for constraint in &$schema.constraints {
			match constraint {
				$constraint::Min(min) => bounds.0 = Some(min.value as f64),
				$constraint::Max(max) => bounds.1 = Some(max.value as f64),
				$constraint::Step(step) => bounds.2 = Some(step.value as f64),
			}
		}
		bounds
	}};
}

/// One dispatched leaf. Returns a [`Snippet`] because each arm builds a
/// differently-shaped tree, which is also what lets the struct arm recurse.
///
/// `depth` counts *struct nesting* only: a reference hop or an `Optional`
/// unwrap is the same leaf seen more precisely, so neither consumes budget, and
/// depth `0` stays "the form's own top level".
fn schema_field(
	resolver: SchemaResolver<'_>,
	schema: ValueSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	match schema {
		ValueSchema::Bool(_) => bool_field(field, label),
		ValueSchema::I64(schema) => {
			number_field(number_bounds!(schema, I64Constraint), field, label)
		}
		ValueSchema::U64(schema) => {
			number_field(number_bounds!(schema, U64Constraint), field, label)
		}
		ValueSchema::F64(schema) => {
			number_field(number_bounds!(schema, F64Constraint), field, label)
		}
		ValueSchema::String(schema) => string_field(schema, field, label),
		// a payload carries data no `<option>` value can express, so only an
		// all-unit enum is a select
		ValueSchema::Enum(schema)
			if schema
				.variants
				.iter()
				.all(|variant| variant.payload.is_none()) =>
		{
			select_field(schema, field, label)
		}
		// null is one of the values, which a control's empty state already is
		ValueSchema::Optional(inner) => {
			schema_field(resolver, *inner, field, label, depth)
		}
		ValueSchema::Reference(ref name) => {
			match resolver.schema(name).cloned() {
				Some(resolved) => {
					schema_field(resolver, resolved, field, label, depth)
				}
				// still arriving, or never coming: loud, not silently empty
				None => uneditable(schema, field, label),
			}
		}
		ValueSchema::Struct(_) if depth >= MAX_DEPTH => {
			uneditable(schema, field, label)
		}
		ValueSchema::Struct(schema) => {
			struct_field(resolver, schema, field, label, depth)
		}
		_ => uneditable(schema, field, label),
	}
}

/// The boolean arm: the [`Checkbox`], the one control that produces a `Bool`.
fn bool_field(field: FieldRef, label: Option<String>) -> Snippet {
	labeled(
		label,
		widget(Checkbox {
			name: PropOpt(Some(field.field_path.to_string())),
			field: PropOpt(Some(field)),
		}),
	)
}

/// The numeric arm: a [`NumberField`] carrying whichever constraint bounds the
/// schema declares.
fn number_field(
	(min, max, step): (Option<f64>, Option<f64>, Option<f64>),
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	labeled(
		label,
		widget(NumberField {
			name: PropOpt(Some(field.field_path.to_string())),
			field: PropOpt(Some(field)),
			min: PropOpt(min),
			max: PropOpt(max),
			step: PropOpt(step),
			..default()
		}),
	)
}

/// The string arm: a [`TextArea`] for multiline prose, else a [`TextField`],
/// masked when the schema marks the value sensitive.
fn string_field(
	schema: StringSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let name = PropOpt(Some(field.field_path.to_string()));
	let widget = match schema.multiline {
		true => widget(TextArea {
			name,
			field: PropOpt(Some(field)),
			..default()
		}),
		false => widget(TextField {
			name,
			field: PropOpt(Some(field)),
			sensitive: schema.sensitive,
			..default()
		}),
	};
	labeled(label, widget)
}

/// The unit-enum arm: a [`Select`] with one `<option>` per variant, its value
/// the variant name (the serde form of a unit variant).
fn select_field(
	schema: EnumSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let name = field.field_path.to_string();
	let options = schema
		.variants
		.into_iter()
		.map(|variant| {
			let variant = variant.name.to_string();
			rsx! { <option value=variant.clone()>{variant}</option> }
				.any_snippet()
		})
		.collect::<Vec<_>>();
	labeled(label, rsx! {
		<Select field={field} name={name}>{options}</Select>
	})
}

/// The struct arm: one nested control per named field, each [`FieldRef`]
/// extending this one's path and labelled by the field's label hint (else its
/// key). The form's own top level *is* the group, so only a nested struct wraps
/// its rows in a disclosure.
fn struct_field(
	resolver: SchemaResolver<'_>,
	schema: StructSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let rows = schema
		.fields
		.into_iter()
		.map(|named| {
			let child = FieldRef {
				document: field.document.clone(),
				field_path: field.field_path.with_pushed(named.key.clone()),
				on_missing: default(),
			};
			let label = named
				.label
				.map(|label| label.to_string())
				.unwrap_or_else(|| named.key.to_string());
			schema_field(resolver, named.schema, child, Some(label), depth + 1)
		})
		.collect::<Vec<_>>();
	// the form's own top level is already the group
	if depth == 0 {
		return labeled(None, rows);
	}
	let title = label
		.or_else(|| schema.name.map(|name| name.to_string()))
		.unwrap_or_else(|| field.field_path.to_string());
	rsx! {
		<details open>
			<summary>{title}</summary>
			{rows}
		</details>
	}
	.any_snippet()
}

/// The read-only leaf for a schema with no control: the value is shown but not
/// editable, and the gap is announced where it is built.
fn uneditable(
	schema: ValueSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	warn!(
		"DynamicForm: no control edits a `{}` field at `{}`, rendering it read-only",
		schema.variant_name(),
		field.field_path
	);
	labeled(label, rsx! {
		<span {UneditableField(schema)}>{field}</span>
	})
}

/// Wrap a control in a `<label>` row (the key above its value, per the form
/// rules), or pass it through unlabelled.
fn labeled<M>(label: Option<String>, widget: impl IntoSnippet<M>) -> Snippet {
	match label {
		Some(label) => rsx! { <label>{label}{widget}</label> }.any_snippet(),
		None => Snippet::from_bundle(widget.into_snippet()),
	}
}

/// Lift a constructed widget into a child-position [`Snippet`], the
/// struct-literal twin of an `rsx!` `<Widget/>` tag — reached for here because a
/// dispatched arm passes `Option`s straight into optional props, which a tag
/// cannot express.
fn widget(template: impl BuildTemplate) -> Snippet {
	Snippet::from_bundle(template.into_snippet_bundle())
}

#[cfg(test)]
mod test {
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Render a `DynamicForm` for `schema` bound to a `"field"` key.
	fn render(schema: ValueSchema) -> String {
		test_ext::render_html(rsx! {
			<DynamicForm schema={schema} field={FieldRef::new("field")}/>
		})
	}

	#[beet_core::test]
	fn bool_dispatches_to_a_checkbox() {
		render(ValueSchema::Bool(default()))
			.xpect_contains("type=\"checkbox\"");
	}

	/// A number carries its constraint bounds onto the control, so the widget
	/// enforces what the schema declares without restating it.
	#[beet_core::test]
	fn numbers_dispatch_to_a_number_field_with_bounds() {
		render(ValueSchema::I64(I64Schema {
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
		render(ValueSchema::String(default())).xpect_contains("type=\"text\"");
		render(ValueSchema::String(StringSchema::default().multiline()))
			.xpect_contains("<textarea");
		render(ValueSchema::String(StringSchema::default().sensitive()))
			.xpect_contains("type=\"password\"");
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	enum Role {
		Engineer,
		Designer,
	}

	#[beet_core::test]
	fn a_unit_enum_dispatches_to_a_select() {
		render(ValueSchema::of::<Role>())
			.xpect_contains("<select")
			.xpect_contains("<option value=\"Engineer\"")
			.xpect_contains("<option value=\"Designer\"");
	}

	/// A shape no control can produce a valid value for renders read-only and
	/// marked, rather than as a text box that would write a lie into the
	/// document. An unresolved reference lands here too, so a schema that never
	/// arrived is loud rather than an empty form.
	#[beet_core::test]
	fn structural_shapes_are_uneditable() {
		for schema in [
			ValueSchema::List(default()),
			ValueSchema::Map(default()),
			ValueSchema::Any,
			ValueSchema::Reference("NotRegistered".into()),
		] {
			let mut world = world_ext::ui_world();
			let root = world
				.spawn_template(rsx! {
					<DynamicForm schema={schema} field={FieldRef::new("field")}/>
				})
				.unwrap()
				.id();
			world
				.query_once::<&UneditableField>()
				.into_iter()
				.count()
				.xpect_eq(1);
			let html = test_ext::render_world(&mut world, root);
			html.clone().xnot().xpect_contains("<input");
			html.xnot().xpect_contains("<textarea");
		}
	}

	#[derive(Reflect)]
	struct Profile {
		name: String,
		count: i64,
		active: bool,
	}

	/// The top level is the form's own rows; a nested struct is a disclosure
	/// group, one labelled control per field dispatched by its own schema.
	#[beet_core::test]
	fn a_struct_recurses_with_labels() {
		let html = render(ValueSchema::Struct(StructSchema {
			name: Some("Outer".into()),
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"profile",
				ValueSchema::of::<Profile>(),
			)],
		}));
		html.clone()
			// the top-level field is a row, its nested struct a disclosure
			.xpect_contains("<details open")
			.xpect_contains("<summary>profile</summary>")
			.xpect_contains("name")
			.xpect_contains("count")
			.xpect_contains("type=\"text\"")
			.xpect_contains("type=\"number\"")
			.xpect_contains("type=\"checkbox\"");
		// the nested paths are the leaves' names, so a submit gathers them whole
		html.clone().xpect_contains("name=\"field.profile.name\"");
		// ...and the top level is the form itself, not a group inside it
		html.xnot().xpect_contains("<summary>Outer");
	}

	/// A field's label hint replaces its key as the visible label; the key is
	/// still what the path (and so the binding) uses.
	#[beet_core::test]
	fn a_label_hint_replaces_the_key() {
		render(ValueSchema::Struct(StructSchema {
			name: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("name", ValueSchema::String(default()))
					.with_label("Display Name"),
			],
		}))
		.xpect_contains("Display Name")
		.xpect_contains("name=\"field.name\"");
	}

	/// A reference resolves against the registry and dispatches to the schema it
	/// names, which is what lets a data document compose `Reference("TodoItem")`
	/// and still generate a real form.
	#[beet_core::test]
	fn a_reference_resolves_through_the_registry() {
		let mut world = world_ext::ui_world();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("Role", ValueSchema::of::<Role>());
		let root = world
			.spawn_template(rsx! {
				<DynamicForm
					schema={ValueSchema::Reference("Role".into())}
					field={FieldRef::new("role")}
				/>
			})
			.unwrap()
			.id();
		test_ext::render_world(&mut world, root)
			.xpect_contains("<option value=\"Engineer\"");
	}

	/// One control per editable leaf, each binding its own path and nothing
	/// else: the binding contract `form_controls::conformance` fences, now
	/// generated from a schema rather than authored by hand.
	#[beet_core::test]
	fn each_leaf_binds_its_own_path() {
		let mut world = world_ext::ui_world();
		world
			.spawn_template(rsx! {
				<DynamicForm schema={ValueSchema::of::<Profile>()}/>
			})
			.unwrap();
		world.update_local();
		world
			.query_once::<(&Element, &FieldRef)>()
			.into_iter()
			.map(|(element, field)| {
				(field.field_path.to_string(), element.tag().to_string())
			})
			.collect::<Vec<_>>()
			.xtap(|bindings| bindings.sort())
			.xpect_eq(vec![
				("active".to_string(), "input".to_string()),
				("count".to_string(), "input".to_string()),
				("name".to_string(), "input".to_string()),
			]);
	}

	/// The generated form is an ordinary bound form: an edit reaches its own
	/// leaf of the document and no sibling, and the form holds no copy of it.
	#[beet_core::test]
	fn an_edit_reaches_its_own_leaf() {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div><DynamicForm schema={ValueSchema::of::<Profile>()}/></div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(value!({
			"name": "ada", "count": 1, "active": false
		})));
		world.update_local();

		// the checkbox is the only control that can produce a `Bool`
		let checkbox = world
			.query_once::<(Entity, &CheckboxInput)>()
			.into_iter()
			.map(|(entity, _)| entity)
			.next()
			.unwrap();
		*world.entity_mut(checkbox).get_mut::<Value>().unwrap() =
			Value::Bool(true);
		world.update_local();

		world
			.entity(root)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "name": "ada", "count": 1, "active": true }));
	}

	/// The whole form gathers on submit as a *typed* map: a checkbox submits a
	/// `Bool` and a number an `Int`, which is what lets a schema-driven
	/// submission validate against the schema that generated it.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn submit_gathers_typed_values() {
		let mut app = test_ext::form_app();
		let captured = Store::new(None::<Value>);
		app.world_mut().add_observer(move |ev: On<Submit>| {
			captured.set(Some(ev.values.clone()));
		});
		let root = app
			.world_mut()
			.spawn_template(rsx! {
				<div>
					<DynamicForm schema={ValueSchema::of::<Profile>()}>
						<Button>"Save"</Button>
					</DynamicForm>
				</div>
			})
			.unwrap()
			.id();
		app.world_mut().entity_mut(root).insert(Document::new(
			value!({ "name": "ada", "count": 7, "active": true }),
		));
		app.update();

		let button = test_ext::element(&mut app, "button");
		test_ext::click(&mut app, button);
		let values = captured.get().unwrap();
		values.get("name").unwrap().xpect_eq(Value::str("ada"));
		values.get("count").unwrap().xpect_eq(Value::Int(7));
		values.get("active").unwrap().xpect_eq(Value::Bool(true));
	}

	/// Typing into a generated control writes through its own extended path into
	/// the right document slot, on the terminal, driven by real key input.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn typing_reaches_a_nested_document_path() {
		let mut app = test_ext::form_app();
		let root = app
			.world_mut()
			.spawn_template(rsx! {
				<div>
					<DynamicForm
						schema={ValueSchema::of::<Profile>()}
						field={FieldRef::new("profile")}
					/>
				</div>
			})
			.unwrap()
			.id();
		app.world_mut()
			.entity_mut(root)
			.insert(Document::new(value!({
				"profile": { "name": "", "count": 0, "active": false }
			})));
		app.update();

		// the first input in document order is the `name` text field
		let (window, _) = test_ext::focus_element(&mut app, "input");
		test_ext::type_text(&mut app, window, "hi");

		app.world()
			.get::<Document>(root)
			.unwrap()
			.get_field::<String>(&[
				FieldSegment::key("profile"),
				FieldSegment::key("name"),
			])
			.unwrap()
			.xpect_eq("hi".to_string());
	}
}
