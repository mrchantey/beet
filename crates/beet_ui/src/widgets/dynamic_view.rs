//! [`DynamicView`]: a `<table>` generated from a [`ValueSchema`], one column per
//! item field and one row per list item.
//!
//! The read side of [`DynamicForm`]'s contract: the columns come from the item
//! schema exactly as the form's controls do, and each cell binds one
//! `(document, field path)` through its own [`FieldRef`], so a row shows what
//! the document holds and nothing here holds a copy of it. Rows are a
//! [`ReactiveChildren`] over the list field, so an appended item appears without
//! anything rebuilding the table.
use crate::prelude::*;
use beet_core::prelude::*;

/// A table of the list at `field`, its columns generated from `schema`:
///
/// - a `List` schema (or a `Reference`/`Optional` wrapping one) contributes its
///   item schema; an item schema may also be passed directly
/// - a `Struct` item is one column per field, headed by the field's label hint
///   (else its key), each cell bound to that field of its row
/// - any other item is a single column of the item itself, headed by the list
///   field's own name
///
/// Cells are read-only text bound to their field, so an outside edit reflows
/// into the table. Editing a row is a [`DynamicForm`] over that row.
///
/// ```rsx
/// <DynamicView schema={ValueSchema::of::<Vec<TodoItem>>()} field={FieldRef::new("items")}/>
/// ```
#[template(system)]
pub fn DynamicView(
	/// The schema of the list, or of one of its items.
	#[prop(required)]
	schema: ValueSchema,
	/// The list field, one row per item.
	#[prop(required)]
	field: FieldRef,
	/// Draw a full cell grid rather than only horizontal row rules.
	#[prop]
	vertical_lines: bool,
	/// The by-name registry a [`ValueSchema::Reference`] resolves against;
	/// absent until [`DocumentPlugin`] has initialized it, which leaves an
	/// item schema unresolved and falls back to the single row-value column.
	schemas: Option<Res<SchemaRegistry>>,
) -> impl Bundle {
	let resolver = schemas
		.as_deref()
		.map(|schemas| SchemaResolver::default().with_schemas(schemas))
		.unwrap_or_default();
	let columns = columns(resolver, schema, &field);
	let headers = columns
		.iter()
		.map(|column| {
			rsx! { <th>{column.label.to_string()}</th> }.any_snippet()
		})
		.collect::<Vec<_>>();
	let mut class_set = Classes::new([classes::TABLE]);
	if vertical_lines {
		class_set.insert_class(classes::TABLE_VERTICAL_BORDERS);
	}
	rsx! {
		<table {class_set}>
			<thead><tr>{headers}</tr></thead>
			// the reactive binding sits on the `<tbody>` itself, so each item's
			// row is a direct child of it; `Table`'s slots cannot express that.
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
	/// The item field this column shows, `None` being the item itself (a list
	/// of scalars).
	key: Option<SmolStr>,
	/// The header text.
	label: SmolStr,
}

/// The columns `schema` describes: a struct item's fields, else the single
/// column of the item itself.
fn columns(
	resolver: SchemaResolver<'_>,
	schema: ValueSchema,
	field: &FieldRef,
) -> Vec<Column> {
	match item_schema(resolver, schema) {
		Some(ValueSchema::Struct(schema)) => schema
			.fields
			.into_iter()
			.map(|named| Column {
				key: Some(named.key.clone()),
				label: named.label.unwrap_or(named.key),
			})
			.collect(),
		// a list of scalars, or a schema that never resolved: one column of the
		// row itself, named for the list it came from
		_ => vec![Column {
			key: None,
			label: field
				.field_path
				.iter()
				.next_back()
				.map(|segment| segment.to_string())
				.unwrap_or_else(|| "value".to_string())
				.into(),
		}],
	}
}

/// The schema of one item: a `List`'s item schema, seen through any number of
/// `Reference`/`Optional` hops, else the schema itself (an item schema passed
/// directly). `None` when a reference never resolved.
fn item_schema(
	resolver: SchemaResolver<'_>,
	schema: ValueSchema,
) -> Option<ValueSchema> {
	match schema {
		ValueSchema::List(schema) => item_schema(resolver, *schema.item),
		ValueSchema::Optional(inner) => item_schema(resolver, *inner),
		ValueSchema::Reference(name) => {
			item_schema(resolver, resolver.schema(&name).cloned()?)
		}
		schema => Some(schema),
	}
}

/// One row: a `<tr>` of cells, each seeded with the item's current value so a
/// fresh row paints on its first frame, and bound so a later edit re-syncs it.
///
/// The [`ReactiveChildren`] scopes the row to `items[index]`, so a cell's path
/// is relative to its own item.
fn row(columns: &[Column], item: &Value) -> OnSpawn {
	let cells = columns
		.iter()
		.map(|column| {
			let (seed, path) = match &column.key {
				Some(key) => (
					item.get(key).cloned().unwrap_or_default(),
					FieldPath::new([key.clone()]),
				),
				None => (item.clone(), FieldPath::default()),
			};
			rsx! { <td>{(seed, FieldRef::new(path))}</td> }.any_snippet()
		})
		.collect::<Vec<_>>();
	OnSpawn::insert(rsx! { <tr>{cells}</tr> })
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

	/// A world holding `items` as a document, with a `DynamicView` over it.
	fn view(items: Value) -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicView
						schema={ValueSchema::of::<Vec<TodoItem>>()}
						field={FieldRef::new("items")}
					/>
				</div>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(value!({ "items": items })));
		world.update_local();
		(world, root)
	}

	/// The columns come from the item schema and the rows from the document, so
	/// a table of a typed list needs nothing list-specific authored.
	#[beet_core::test]
	fn columns_come_from_the_schema_and_rows_from_the_document() {
		let (mut world, root) = view(value!([
			{ "label": "buy milk", "done": false },
			{ "label": "walk dog", "done": true }
		]));
		test_ext::render_world(&mut world, root)
			.xpect_contains("<th>label</th>")
			.xpect_contains("<th>done</th>")
			.xpect_contains("buy milk")
			.xpect_contains("walk dog")
			.xpect_contains("true");
	}

	/// Each cell binds its own row's field: the rows carry the list's item paths,
	/// one binding per cell, and no cell holds a copy of the document.
	#[beet_core::test]
	fn each_cell_binds_its_own_row() {
		let (mut world, _) = view(value!([
			{ "label": "buy milk", "done": false },
			{ "label": "walk dog", "done": true }
		]));
		world
			.query_once::<(&ResolvedFieldPath, &Value)>()
			.into_iter()
			.filter(|(resolved, _)| resolved.field_path.iter().count() == 3)
			.map(|(resolved, value)| {
				(resolved.field_path.to_string(), value.to_string())
			})
			.collect::<Vec<_>>()
			.xtap(|bindings| bindings.sort())
			.xpect_eq(vec![
				("items.[0].done".to_string(), "false".to_string()),
				("items.[0].label".to_string(), "buy milk".to_string()),
				("items.[1].done".to_string(), "true".to_string()),
				("items.[1].label".to_string(), "walk dog".to_string()),
			]);
	}

	/// Appending to the list appends a row: the view rides the document, so
	/// nothing has to tell it the list changed.
	#[beet_core::test]
	fn an_appended_item_appends_a_row() {
		let (mut world, root) =
			view(value!([{ "label": "buy milk", "done": false }]));
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

	/// A list of scalars is a single column of the items themselves, headed by
	/// the field it came from.
	#[beet_core::test]
	fn a_scalar_list_is_one_column() {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! {
				<div>
					<DynamicView
						schema={ValueSchema::of::<Vec<String>>()}
						field={FieldRef::new("tags")}
					/>
				</div>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(value!({ "tags": ["red", "green"] })));
		world.update_local();
		test_ext::render_world(&mut world, root)
			.xpect_contains("<th>tags</th>")
			.xpect_contains("red")
			.xpect_contains("green");
	}

	/// The table paints as a column-aligned grid on the terminal, the surface it
	/// ships on first.
	#[beet_core::test]
	fn renders_on_the_terminal() {
		test_ext::render_charcell(
			40,
			Document::new(
				value!({ "items": [{ "label": "buy milk", "done": false }] }),
			),
			rsx! {
				<DynamicView
					schema={ValueSchema::of::<Vec<TodoItem>>()}
					field={FieldRef::new("items")}
				/>
			},
		)
		.xpect_contains("label")
		.xpect_contains("buy milk");
	}
}
