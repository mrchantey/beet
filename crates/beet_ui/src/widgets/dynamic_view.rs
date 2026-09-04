//! [`DynamicView`]: the read representation of any value, generated from a
//! [`ValueSchema`].
//!
//! The read side of [`DynamicForm`]'s contract, and the same walk: a schema kind
//! picks the presentation, a struct recurses into its fields, and every leaf
//! binds one `(document, field path)` through its own [`FieldRef`], so a view
//! shows what the document holds and nothing here holds a copy of it.
//!
//! Where the form has a control per leaf, the view has a line: a scalar reads as
//! `key: value`, a struct as a titled block between rules, and a list as one
//! line per item — or, when its items are structs, as the column-aligned table
//! whose headers are the item's own fields. Reading is total where editing is
//! not: every kind renders, because a [`Value`] is already text, so the view
//! needs no [`UneditableField`] to fall back to.
//!
//! Three things are reactive, at three grains: a leaf's value through its own
//! binding, a list's rows through [`ReactiveChildren`], and the layout itself
//! through [`SchemaRebuild`], so a committed schema edit grows the table a
//! column.
use crate::prelude::*;
use beet_core::prelude::*;

/// Cap on nested [`ValueSchema::Struct`] recursion, the twin of
/// [`DynamicForm`]'s: a schema graph is finite by construction, so this is a
/// defensive bound, and a deeper subtree reads as its raw value.
const MAX_DEPTH: usize = 8;

/// A read-only view of the document value at `field`, laid out from `schema`:
///
/// - a scalar (`Bool`/`I64`/`U64`/`F64`/`String`/`Bytes`/`Entity`/`Enum`/..)
///   reads as a `key: value` line, the key being the field's label hint else its
///   key
/// - a `Struct` reads as its title over an open and close rule, one nested view
///   per field, each [`FieldRef`] extending this one's path. At the top level
///   the fields are the view's own rows, since the view is already the group.
/// - a `List` of structs reads as a table, one column per item field and one row
///   per item; a `List` of anything else as one value line per item. Both ride a
///   [`ReactiveChildren`] over the list field, so an appended item appears
///   without anything rebuilding the view.
/// - `Optional` reads as its inner schema, and `Reference` as the schema it
///   names, resolved against the [`SchemaRegistry`]
///
/// Editing a value is a [`DynamicForm`] over the same field.
///
/// ```rsx
/// <DynamicView schema={ValueSchema::of::<Vec<TodoItem>>()} field={FieldRef::new("items")}/>
/// ```
#[template(system)]
pub fn DynamicView(
	/// The schema of the value this view reads. Omitted, the view reads the
	/// schema the bound document declares at `field`, which is the authored
	/// form for a document loaded out of a store.
	#[prop]
	schema: Option<ValueSchema>,
	/// The document field the view reads: the path every generated leaf's own
	/// [`FieldRef`] extends. Defaults to the whole document.
	#[prop]
	field: FieldRef,
	/// Draw a full cell grid on a table of structs, rather than only horizontal
	/// row rules.
	#[prop]
	vertical_lines: bool,
	/// The by-name registry a [`SchemaRef::Name`] resolves against;
	/// absent until [`DocumentPlugin`] has initialized it, which leaves every
	/// reference unresolved and reads its value raw.
	schemas: Option<Res<SchemaRegistry>>,
) -> impl Bundle {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	// the laid-out value is one generation, respawned when a committed schema
	// edit changes what this schema resolves to (a table gaining a column)
	let rows = {
		let field = field.clone();
		move |resolver: SchemaResolver, schema: &ValueSchema| {
			let cx = ViewCx {
				resolver,
				vertical_lines,
			};
			view_field(cx, schema, field.clone(), None, 0)
		}
	};
	let source = match schema {
		Some(schema) => SchemaSource::Authored(schema),
		None => SchemaSource::Document(field),
	};
	rsx! {
		<div>{SchemaRebuild::new(resolver, source, rows).holder(resolver)}</div>
	}
}

/// The recursion's carried state: the registries a reference resolves against,
/// and the chrome the author chose. `Copy`, so an arm passes it on rather than
/// threading each piece.
#[derive(Clone, Copy)]
struct ViewCx<'a> {
	resolver: SchemaResolver<'a>,
	vertical_lines: bool,
}

/// One dispatched leaf. Returns a [`Snippet`] because each arm builds a
/// differently-shaped tree, which is also what lets the struct arm recurse.
///
/// `depth` counts *struct nesting* only, exactly as [`DynamicForm`]'s walk does:
/// a reference hop or an `Optional` unwrap is the same value seen more
/// precisely, so neither consumes budget, and depth `0` stays "the view's own
/// top level".
fn view_field<'a>(
	cx: ViewCx<'a>,
	schema: &'a ValueSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	match schema {
		// null is one of the values, which an empty line already reads as
		ValueSchema::Optional(inner) => {
			view_field(cx, inner, field, label, depth)
		}
		// the registry's schema is borrowed, never copied out
		ValueSchema::Ref(SchemaRef::Name(name)) => {
			match cx.resolver.schema(name) {
				Some(resolved) => view_field(cx, resolved, field, label, depth),
				// still arriving, or never coming: the value still reads
				None => value_row(field, label),
			}
		}
		ValueSchema::Struct(_) if depth >= MAX_DEPTH => value_row(field, label),
		ValueSchema::Struct(schema) => {
			struct_block(cx, schema, field, label, depth)
		}
		ValueSchema::List(schema) => list_block(cx, schema, field, label),
		// every other kind is a value the document already renders as text
		_ => value_row(field, label),
	}
}

/// The scalar arm: a `key: value` line, the value bound so an outside edit
/// reflows into it. A text node carries no element, so nothing can focus or type
/// into it — the view is read-only by construction, not by convention.
fn value_row(field: FieldRef, label: Option<String>) -> Snippet {
	match label {
		Some(label) => rsx! {
			<div>{format!("{label}: ")}{field}</div>
		}
		.any_snippet(),
		None => rsx! { <div>{field}</div> }.any_snippet(),
	}
}

/// The struct arm: the title over an open and close rule, one nested view per
/// named field, each [`FieldRef`] extending this one's path and labelled by the
/// field's label hint (else its key). The view's own top level *is* the group,
/// so only a nested struct draws the chrome.
fn struct_block<'a>(
	cx: ViewCx<'a>,
	schema: &'a StructSchema,
	field: FieldRef,
	label: Option<String>,
	depth: usize,
) -> Snippet {
	let rows = schema
		.fields
		.iter()
		.map(|named| {
			let child = FieldRef {
				document: field.document.clone(),
				field_path: field.field_path.with_pushed(named.key.clone()),
				on_missing: default(),
			};
			let label = named.label.as_ref().unwrap_or(&named.key).to_string();
			view_field(cx, &named.schema, child, Some(label), depth + 1)
		})
		.collect::<Vec<_>>();
	if depth == 0 {
		return flatten(rows);
	}
	let title = label
		.or_else(|| schema.name.as_ref().map(|name| name.to_string()))
		.unwrap_or_else(|| field.field_path.to_string());
	titled(Some(title), rsx! { <hr/>{rows}<hr/> })
}

/// The list arm: a table when the items are structs, else one value line per
/// item.
fn list_block<'a>(
	cx: ViewCx<'a>,
	schema: &'a ListSchema,
	field: FieldRef,
	label: Option<String>,
) -> Snippet {
	match resolved(cx.resolver, &schema.item) {
		Some(ValueSchema::Struct(item)) => {
			titled(label, item_table(cx, item, field))
		}
		_ => titled(label, value_list(field)),
	}
}

/// Follow `Optional`/`Reference` hops to the schema they name, `None` when a
/// reference never resolved. The two arms [`view_field`] recurses through
/// without changing what is being described.
fn resolved<'a>(
	resolver: SchemaResolver<'a>,
	schema: &'a ValueSchema,
) -> Option<&'a ValueSchema> {
	match schema {
		ValueSchema::Optional(inner) => resolved(resolver, inner),
		ValueSchema::Ref(SchemaRef::Name(name)) => {
			resolved(resolver, resolver.schema(name)?)
		}
		schema => Some(schema),
	}
}

/// A list of non-struct items: one value line per item, each line's own
/// [`FieldRef`] empty so it reads the item itself within the row's scope.
fn value_list(field: FieldRef) -> Snippet {
	rsx! {
		<div {(
			field,
			ReactiveChildren::new(|_index, item| {
				OnSpawn::insert(rsx! {
					<div>{(item.clone(), FieldRef::default())}</div>
				})
			}),
		)}/>
	}
}

/// A list of structs: a table whose columns are the item's own fields.
///
/// The reactive binding sits on the `<tbody>` itself, so each item's row is a
/// direct child of it; [`Table`]'s slots cannot express that, which is why this
/// builds its own `<table>` rather than composing the widget.
fn item_table(cx: ViewCx<'_>, item: &StructSchema, field: FieldRef) -> Snippet {
	let columns = item
		.fields
		.iter()
		.map(|named| Column {
			key: named.key.clone(),
			label: named.label.clone().unwrap_or_else(|| named.key.clone()),
		})
		.collect::<Vec<_>>();
	let headers = columns
		.iter()
		.map(|column| {
			rsx! { <th>{column.label.to_string()}</th> }.any_snippet()
		})
		.collect::<Vec<_>>();
	let mut class_set = Classes::new([classes::TABLE]);
	if cx.vertical_lines {
		class_set.insert_class(classes::TABLE_VERTICAL_BORDERS);
	}
	rsx! {
		<table {class_set}>
			<thead><tr>{headers}</tr></thead>
			<tbody {(
				field,
				ReactiveChildren::new(move |_index, item| row(&columns, item)),
			)}/>
		</table>
	}
}

/// One generated column: the item field its cells bind, and the header text
/// above them.
#[derive(Clone)]
struct Column {
	/// The item field this column shows.
	key: SmolStr,
	/// The header text.
	label: SmolStr,
}

/// One table row: a `<tr>` of cells, each seeded with the item's current value
/// so a fresh row paints on its first frame, and bound so a later edit re-syncs
/// it.
///
/// The [`ReactiveChildren`] scopes the row to `items[index]`, so a cell's path
/// is relative to its own item.
fn row(columns: &[Column], item: &Value) -> OnSpawn {
	let cells = columns
		.iter()
		.map(|column| {
			let seed = item.get(&column.key).cloned().unwrap_or_default();
			let field = FieldRef::new(FieldPath::new([column.key.clone()]));
			rsx! { <td>{(seed, field)}</td> }.any_snippet()
		})
		.collect::<Vec<_>>();
	OnSpawn::insert(rsx! { <tr>{cells}</tr> })
}

/// Head `content` with the name of the field it reads, or pass it through
/// unlabelled (the view's own top level, which is already the group).
fn titled<M>(label: Option<String>, content: impl IntoSnippet<M>) -> Snippet {
	match label {
		Some(label) => rsx! {
			<div><div><strong>{label}</strong></div>{content}</div>
		}
		.any_snippet(),
		None => flatten(content),
	}
}

/// Lift rows into a [`Snippet`] with no wrapper of their own: a tag-less node is
/// transparent to both renderers, so the rows lay out as siblings of whatever
/// they land beside.
fn flatten<M>(rows: impl IntoSnippet<M>) -> Snippet {
	Snippet::from_bundle(rows.into_snippet())
}

#[cfg(test)]
mod test {
	use super::super::test_ext;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	struct TodoItem {
		label: String,
		done: bool,
	}

	#[derive(Reflect)]
	struct Settings {
		is_enabled: bool,
		retries: i64,
	}

	#[derive(Reflect)]
	struct Account {
		name: String,
		settings: Settings,
	}

	/// Build a view of `schema` over `document`, settled.
	fn build(
		schema: ValueSchema,
		field: FieldRef,
		document: Value,
	) -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div><DynamicView schema={schema} field={field}/></div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(document));
		world.update_local();
		(world, root)
	}

	/// [`build`], rendered to HTML.
	fn view(schema: ValueSchema, field: FieldRef, document: Value) -> String {
		let (mut world, root) = build(schema, field, document);
		test_ext::render_world(&mut world, root)
	}

	/// The document every scalar test reads.
	fn settings() -> Value { value!({ "is_enabled": true, "retries": 3 }) }

	/// A scalar reads as a `key: value` line, the key being the field's own name.
	#[beet_core::test]
	fn a_scalar_reads_as_a_key_value_line() {
		view(
			ValueSchema::of::<Settings>(),
			FieldRef::default(),
			settings(),
		)
		.xpect_contains("is_enabled: ")
		.xpect_contains("true")
		.xpect_contains("retries: ")
		.xpect_contains("3");
	}

	/// The view's own top level is the group, so a top-level struct draws no
	/// title and no rules — only a nested one does.
	#[beet_core::test]
	fn the_top_level_struct_draws_no_chrome() {
		view(
			ValueSchema::of::<Settings>(),
			FieldRef::default(),
			settings(),
		)
		.xnot()
		.xpect_contains("<hr");
	}

	/// The document every nested-struct test reads.
	fn account() -> Value {
		value!({ "name": "ada", "settings": { "is_enabled": true, "retries": 3 } })
	}

	/// A nested struct reads as its title between an open and close rule, one
	/// line per field.
	#[beet_core::test]
	fn a_nested_struct_reads_as_a_titled_block() {
		view(ValueSchema::of::<Account>(), FieldRef::default(), account())
			.xpect_contains("name: ")
			.xpect_contains("<strong>settings</strong>")
			.xpect_contains("<hr")
			.xpect_contains("is_enabled: ");
	}

	/// Every leaf binds its own path and nothing else, the read half of the
	/// binding contract `form_controls::conformance` fences.
	#[beet_core::test]
	fn each_leaf_binds_its_own_path() {
		bindings(build(
			ValueSchema::of::<Account>(),
			FieldRef::default(),
			account(),
		))
		.xpect_eq(vec![
			("name".to_string(), "ada".to_string()),
			("settings.is_enabled".to_string(), "true".to_string()),
			("settings.retries".to_string(), "3".to_string()),
		]);
	}

	/// Every resolved binding in the world as `(path, value)`, sorted (a query
	/// iterates archetypes, not document order).
	fn bindings((mut world, _): (World, Entity)) -> Vec<(String, String)> {
		world
			.query_once::<(&ResolvedFieldPath, &Value)>()
			.into_iter()
			.map(|(resolved, value)| {
				(resolved.field_path.to_string(), value.to_string())
			})
			.collect::<Vec<_>>()
			.xtap(|bindings| bindings.sort())
	}

	/// The document every list test reads.
	fn todos() -> Value {
		value!({ "items": [
			{ "label": "buy milk", "done": false },
			{ "label": "walk dog", "done": true }
		] })
	}

	/// A list of structs reads as a table: the columns come from the item schema
	/// and the rows from the document, so nothing list-specific is authored.
	#[beet_core::test]
	fn a_struct_list_reads_as_a_table() {
		view(
			ValueSchema::of::<Vec<TodoItem>>(),
			FieldRef::new("items"),
			todos(),
		)
		.xpect_contains("<th>label</th>")
		.xpect_contains("<th>done</th>")
		.xpect_contains("buy milk")
		.xpect_contains("walk dog")
		.xpect_contains("true");
	}

	/// Each table cell binds its own row's field.
	#[beet_core::test]
	fn each_cell_binds_its_own_row() {
		bindings(build(
			ValueSchema::of::<Vec<TodoItem>>(),
			FieldRef::new("items"),
			todos(),
		))
		.into_iter()
		// the list field itself binds too; the cells are its leaves
		.filter(|(path, _)| path.contains('['))
		.collect::<Vec<_>>()
		.xpect_eq(vec![
			("items.[0].done".to_string(), "false".to_string()),
			("items.[0].label".to_string(), "buy milk".to_string()),
			("items.[1].done".to_string(), "true".to_string()),
			("items.[1].label".to_string(), "walk dog".to_string()),
		]);
	}

	/// A list of scalars reads as one value line per item, not a one-column
	/// table: the table earns its columns from an item's fields, and a scalar
	/// has none.
	#[beet_core::test]
	fn a_scalar_list_reads_as_lines() {
		view(
			ValueSchema::of::<Vec<String>>(),
			FieldRef::new("tags"),
			value!({ "tags": ["red", "green"] }),
		)
		.xpect_contains("red")
		.xpect_contains("green")
		.xnot()
		.xpect_contains("<table");
	}

	/// Appending to a list appends a row: the view rides the document, so
	/// nothing has to tell it the list changed.
	#[beet_core::test]
	fn an_appended_item_appends_a_row() {
		let (mut world, root) = build(
			ValueSchema::of::<Vec<TodoItem>>(),
			FieldRef::new("items"),
			value!({ "items": [{ "label": "buy milk", "done": false }] }),
		);
		let rows = |world: &mut World| {
			world
				.query_once::<&Element>()
				.into_iter()
				.filter(|element| element.tag() == "tr")
				.count()
		};
		// the header row plus one item row
		rows(&mut world).xpect_eq(2);

		world
			.entity_mut(root)
			.get_mut::<Document>()
			.unwrap()
			.0
			.get_mut("items")
			.unwrap()
			.as_list_mut()
			.unwrap()
			.push(value!({ "label": "walk dog", "done": true }));
		world.update_local();

		rows(&mut world).xpect_eq(3);
		test_ext::render_world(&mut world, root).xpect_contains("walk dog");
	}

	/// A reference resolves against the registry and lays out the schema it
	/// names, which is what lets a data document composing
	/// `List(Reference("TodoItem"))` still generate a real table.
	#[beet_core::test]
	fn a_reference_resolves_through_the_registry() {
		let mut world = world_ext::ui_world();
		world
			.get_resource_or_init::<SchemaRegistry>()
			.insert("TodoItem", ValueSchema::of::<TodoItem>());
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicView
						schema={ValueSchema::List(ListSchema {
							item: Box::new(ValueSchema::reference("TodoItem")),
							..default()
						})}
						field={FieldRef::new("items")}
					/>
				</div>
			})
			.unwrap()
			.id();
		world.entity_mut(root).insert(Document::new(
			value!({ "items": [{ "label": "buy milk", "done": false }] }),
		));
		world.update_local();
		test_ext::render_world(&mut world, root)
			.xpect_contains("<th>label</th>")
			.xpect_contains("buy milk");
	}

	/// The view paints on the terminal, the surface it ships on first: a scalar
	/// line, a nested struct's rule, and its fields.
	#[beet_core::test]
	fn renders_on_the_terminal() {
		test_ext::render_charcell(
			40,
			Document::new(account()),
			rsx! { <DynamicView schema={ValueSchema::of::<Account>()}/> },
		)
		.xpect_contains("name: ada")
		.xpect_contains("settings")
		.xpect_contains("─")
		.xpect_contains("is_enabled: true");
	}
}
