//! [`DynamicForm`]: a `<form>` generated from a [`ValueSchema`], one control
//! per editable leaf.
//!
//! The bridge between the schema layer and the form controls, and the dispatch
//! that chooses between them: a schema kind picks the widget, its hints and
//! constraints supply the widget's bounds, and a composite schema recurses into
//! a group whose [`FieldRef`]s extend the parent's path. The arms themselves
//! live beside this one — [`scalar_field`](super::scalar_field) the leaves,
//! [`composite_field`](super::composite_field) the groups and collections, and
//! [`dependent_field`](super::dependent_field) the enum and sibling-bound ones.
//!
//! It owns no state and performs no writes: every leaf binds one
//! `(document, field path)` through its own [`FieldRef`] and writes only its
//! local [`Value`], so the bidirectional syncs carry the edit and nothing here
//! ever holds a copy of a document. Each control also carries the leaf's path
//! as its `name`, so the whole form gathers as a typed [`Value`] map on
//! [`Submit`] — the commit boundary a transactional edit rides.
//!
//! Where a schema alone does not decide the controls, the *value* does, through
//! a [`ValueRebuild`](super::value_rebuild::ValueRebuild): a list's rows, a
//! map's entries, an enum's payload and a field whose schema a sibling names are
//! all generated from the bound value and regenerated when its shape changes.
//! The schema-decided layout around them is one generation of a
//! [`SchemaRebuild`](super::schema_rebuild::SchemaRebuild), so a committed
//! schema edit regenerates the form while the slot's authored children stay
//! put.
//!
//! Surface-agnostic: the widgets it spawns are ordinary elements, so the same
//! form paints in a terminal and serves as HTML.
use super::composite_field;
use super::dependent_field;
use super::field_layout::labeled;
use super::scalar_field;
use super::schema_rebuild::SchemaRebuild;
use super::schema_rebuild::SchemaSource;
use crate::prelude::*;
use beet_core::prelude::*;

/// Cap on nested composite recursion. A schema graph is finite by construction
/// ([`SchemaResolver`] bounds a reference chain and leaves a cycle unresolved),
/// so this is a defensive bound; a deeper subtree renders as an
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
/// - `Optional` -> its inner schema (a missing value reads as empty)
/// - `Reference` -> the schema it names, resolved against the [`SchemaRegistry`]
/// - `Struct` -> a labelled group, one nested control per field, each
///   [`FieldRef`] extending this one's path. At the top level the fields are the
///   form's own rows; nested, they sit in an open `<details>` disclosure.
/// - `Tuple` -> the same group over its elements, labelled by position
/// - `List` -> one generated control per item, a remove button beside each and
///   an add button after them, appending the item schema's
///   [`default_value_in`](ValueSchema::default_value_in)
/// - `Map` -> the same over its entries, each labelled by its key, added under a
///   key typed beside the add button
/// - `Enum` -> a [`Select`] of variant names; a payload-carrying variant renders
///   its payload's own controls beside it, regenerated when the variant changes
/// - anything else (`Entity`/`Bytes`/`Any`/an unresolved `Reference`) -> a
///   read-only [`UneditableField`], since no control can produce a valid value
///   for it. An `Entity` reference wants the picker item 18 names.
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
	/// The schema of the value this form edits. Omitted, the form edits under
	/// the schema the bound document declares at `field`, which is the authored
	/// form for a document loaded out of a store.
	#[prop]
	schema: Option<ValueSchema>,
	/// The document field the form edits: the path every generated control's
	/// own [`FieldRef`] extends. Defaults to the whole document.
	#[prop]
	field: FieldRef,
	/// The by-name registry a [`SchemaRef::Name`] resolves against;
	/// absent until [`DocumentPlugin`] has initialized it, which defers every
	/// reference to an [`UneditableField`] exactly as an unregistered name does.
	schemas: Option<Res<SchemaRegistry>>,
) -> impl Bundle {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	// the controls are one generation, respawned when a committed schema edit
	// changes what this schema resolves to; the slot's children are its siblings
	let controls = {
		let field = field.clone();
		move |resolver: SchemaResolver, schema: &ValueSchema| {
			schema_field(resolver, schema, field.clone(), None, 0)
		}
	};
	let source = match schema {
		Some(schema) => SchemaSource::Authored(schema),
		None => SchemaSource::Document(field),
	};
	rsx! {
		<Form>
			{SchemaRebuild::new(resolver, source, controls).holder(resolver)}
			<Slot/>
		</Form>
	}
}

/// Marks a [`DynamicForm`] leaf whose schema has no editing widget, naming the
/// kind that found none (`"Entity"`, an unresolved `"Ref"`).
///
/// The leaf still renders, read-only, so a form keeps its shape with only the
/// editing missing — the same bargain an unregistered tag strikes. The mark sits
/// on the bound leaf itself, so one `(&FieldRef, &UneditableField)` query
/// answers both which leaves lost their control and where each one binds. That
/// query is the reason this is the one generated-form type the crate exports:
/// an app auditing which of its fields it cannot edit reads the mark, while the
/// machinery that placed it stays inside the widget set.
///
/// It carries the *kind*, not the schema. A `#[template]` expands away at build,
/// so nothing survives it holding the schema this form walked, and a copy here
/// would be an unowned second one; the authoritative per-leaf schema arrives
/// from the document side instead, seeded onto every bound field by
/// `sync_schema` from the document's own [`DocumentSchema`].
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct UneditableField(pub SmolStr);

/// One dispatched leaf. Returns a [`Snippet`] because each arm builds a
/// differently-shaped tree, which is also what lets the composite arms recurse.
///
/// `depth` counts *composite nesting* — a struct, tuple, list, map or enum
/// payload, the positions the walk can descend through — so a reference hop or
/// an `Optional` unwrap is the same leaf seen more precisely and neither
/// consumes budget, and depth `0` stays "the form's own top level".
pub(super) fn schema_field<'a>(
	resolver: SchemaResolver<'a>,
	schema: &'a ValueSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	match schema {
		ValueSchema::Bool(_) => scalar_field::bool_field(field, label),
		ValueSchema::I64(schema) => {
			scalar_field::i64_field(schema, field, label)
		}
		ValueSchema::U64(schema) => {
			scalar_field::u64_field(schema, field, label)
		}
		ValueSchema::F64(schema) => {
			scalar_field::f64_field(schema, field, label)
		}
		ValueSchema::String(schema) => {
			scalar_field::string_field(schema, field, label)
		}
		// null is one of the values, which a control's empty state already is
		ValueSchema::Optional(inner) => {
			schema_field(resolver, inner, field, label, depth)
		}
		// the registry's schema is borrowed, never copied out: a reference hop
		// dispatches on the schema in place
		ValueSchema::Ref(SchemaRef::Name(name)) => {
			match resolver.schema(name) {
				Some(resolved) => {
					schema_field(resolver, resolved, field, label, depth)
				}
				// still arriving, or never coming: loud, not silently empty
				None => uneditable(schema, field, label),
			}
		}
		// a composite deeper than the budget is the one place the walk gives up
		_ if depth >= MAX_DEPTH && schema.is_composite() => {
			uneditable(schema, field, label)
		}
		// a payload carries data no `<option>` value can express, so only an
		// all-unit enum is a plain select
		ValueSchema::Enum(schema)
			if schema
				.variants
				.iter()
				.all(|variant| variant.payload.is_none()) =>
		{
			dependent_field::unit_enum_field(schema, field, label)
		}
		ValueSchema::Enum(schema) => {
			dependent_field::enum_field(schema, field, label, depth)
		}
		// a field naming a sibling cannot be dispatched until that sibling's
		// value is in hand, so the container binds before it descends
		ValueSchema::Struct(schema)
			if schema
				.fields
				.iter()
				.any(|named| named.schema.binds_a_field()) =>
		{
			dependent_field::bound_struct_field(schema, field, label, depth)
		}
		ValueSchema::Struct(schema) => {
			composite_field::struct_field(resolver, schema, field, label, depth)
		}
		ValueSchema::Tuple(schema) => {
			composite_field::tuple_field(resolver, schema, field, label, depth)
		}
		ValueSchema::List(schema) => {
			composite_field::list_field(resolver, schema, field, label, depth)
		}
		ValueSchema::Map(schema) => {
			composite_field::map_field(resolver, schema, field, label, depth)
		}
		_ => uneditable(schema, field, label),
	}
}

/// The read-only leaf for a schema with no control: a bound text node, so the
/// value still shows but nothing can type into it (a text node carries no
/// element, so the focus path cannot reach it). The gap is announced where it is
/// built and marked where it landed.
fn uneditable(
	schema: &ValueSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	let kind = schema.variant_name();
	warn!(
		"DynamicForm: no control edits a `{kind}` field at `{}`, rendering it read-only",
		field.field_path
	);
	labeled(label, (field, UneditableField(SmolStr::new_static(kind))))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::schema_ui::test_ext;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	enum Role {
		Engineer,
		Designer,
	}

	#[derive(Reflect)]
	struct Profile {
		name: String,
		count: i64,
		active: bool,
	}

	/// A shape no control can produce a valid value for renders read-only and
	/// marked, rather than as a text box that would write a lie into the
	/// document. An unresolved reference lands here too, so a schema that never
	/// arrived is loud rather than an empty form.
	#[beet_core::test]
	fn unreachable_shapes_are_uneditable() {
		for schema in [
			ValueSchema::Entity(default()),
			ValueSchema::Any,
			ValueSchema::reference("NotRegistered"),
		] {
			let kind = schema.variant_name();
			let mut world = world_ext::ui_world();
			let root = world
				.spawn_template(rsx! {
					<DynamicForm schema={schema} field={FieldRef::new("field")}/>
				})
				.unwrap()
				.id();
			world.update_local();
			// the mark names the kind and sits on the leaf that binds
			world
				.query_once::<(&FieldRef, &UneditableField)>()
				.into_iter()
				.map(|(field, mark)| {
					(field.field_path.to_string(), mark.0.to_string())
				})
				.collect::<Vec<_>>()
				.xpect_eq(vec![("field".to_string(), kind.to_string())]);
			let html = test_ext::render_world(&mut world, root);
			html.clone().xnot().xpect_contains("<input");
			html.xnot().xpect_contains("<textarea");
		}
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
					schema={ValueSchema::reference("Role")}
					field={FieldRef::new("role")}
				/>
			})
			.unwrap()
			.id();
		test_ext::render_world(&mut world, root)
			.xpect_contains("<option value=\"Engineer\"");
	}

	/// One control per editable leaf, each binding its own path and nothing
	/// else: the binding contract `controls::form::conformance` fences, now
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

		test_ext::document_of(&mut world, root)
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
