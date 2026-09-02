//! The stock [`FieldRef`] actions, in three families distinguished by what they
//! actually touch:
//!
//! - **self-bound** ([`ReadField`], [`PushField`], ..): read and write the
//!   entity's own [`Value`], which bidi sync mirrors to and from the document a
//!   frame later. Correct for a UI binding, where the local `Value` is what an
//!   input widget writes and the mirror is the whole point.
//! - **`*Typed`** ([`ReadFieldTyped`], [`PushFieldTyped`], ..): generic over the
//!   payload and backed by [`DocumentQuery`], so a write is schema-checked and
//!   lands in the document immediately. BSX has no syntax for a type argument,
//!   so these are Rust-only.
//! - **`*DocField`** ([`ReadDocField`], [`PushDocField`], ..): the same
//!   immediate document access over an untyped [`Value`], and so authorable from
//!   markup. For a caller with no frame to spare, ie a request/response
//!   boundary: `<FieldRoute>` lowers to these, so a `POST` is visible to the
//!   very next `GET` with no settling in between.
//!
//! Every `*DocField` write answers with the field's new value, so a `POST` is
//! self-verifying rather than reading as an empty `null`. [`RemoveAtDocField`]
//! is the exception, answering with the item it removed: that is the one thing
//! a follow-up read can no longer recover.

use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::Typed;

/// An action that increments a numeric field in a document, returning the new value.
///
/// When triggered, this action:
/// 1. Reads the current value from the specified field
/// 2. Increments it by 1
/// 3. Writes the new value back
/// 4. Returns the new value
///
/// If the field doesn't exist or is not an i64, it will be initialized to 1.
///
/// The action is self-bound: it reads and mutates the entity's own [`Value`],
/// which bidi sync mirrors to the document field, rather than going through the
/// document directly.
///
/// # Example
///
/// ```no_run
/// use beet_core::prelude::*;
/// use beet_ui::prelude::*;
///
/// let mut world = AsyncPlugin::world();
/// let field = FieldRef::new("counter");
/// let entity = world.spawn(Increment::bundle(field)).id();
/// ```
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Increment(
	cx: In<ActionContext>,
	mut values: Query<&mut Value>,
) -> Result<i64> {
	let mut value = values.get_mut(cx.id())?;
	let new_value = value.as_i64().unwrap_or(0) + 1;
	*value = Value::Int(new_value);
	Ok(new_value)
}

impl Increment {
	/// Convenience constructor for increment with a field reference and path.
	pub fn bundle(field: FieldRef) -> impl Bundle {
		(field, PathPartial::new("increment"), Increment)
	}
}
/// An action that decrements a numeric field in a document, returning the new value.
///
/// When triggered, this action:
/// 1. Reads the current value from the specified field
/// 2. Decrements it by 1
/// 3. Writes the new value back
/// 4. Returns the new value
///
/// If the field doesn't exist or is not an i64, it will be initialized to -1.
///
/// Self-bound: reads and mutates the entity's own [`Value`].
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Decrement(
	cx: In<ActionContext>,
	mut values: Query<&mut Value>,
) -> Result<i64> {
	let mut value = values.get_mut(cx.id())?;
	let new_value = value.as_i64().unwrap_or(0) - 1;
	*value = Value::Int(new_value);
	Ok(new_value)
}

impl Decrement {
	/// Convenience constructor for decrement with a field reference and path.
	pub fn bundle(field: FieldRef) -> impl Bundle {
		(field, PathPartial::new("decrement"), Decrement)
	}
}
/// An action that adds a value to a numeric field in a document.
///
/// Takes the amount to add as input and returns the new value.
/// If the field doesn't exist or is not an i64, it will be initialized to the provided value.
///
/// Self-bound: reads and mutates the entity's own [`Value`].
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn AddField(
	cx: In<ActionContext<i64>>,
	mut values: Query<&mut Value>,
) -> Result<i64> {
	let mut value = values.get_mut(cx.id())?;
	let new_value = value.as_i64().unwrap_or(0) + cx.input;
	*value = Value::Int(new_value);
	Ok(new_value)
}

impl AddField {
	/// Convenience constructor for add with a field reference and path.
	pub fn bundle(field: FieldRef) -> impl Bundle {
		(field, PathPartial::new("add"), AddField)
	}
}
/// An action that sets a field to a specific [`Value`].
///
/// Takes a [`Value`] as input and stores it in the specified field.
///
/// Self-bound: writes the entity's own [`Value`].
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetField(
	cx: In<ActionContext<Value>>,
	mut values: Query<&mut Value>,
) -> Result<()> {
	let entity = cx.id();
	*values.get_mut(entity)? = cx.input;
	Ok(())
}

/// An action that sets a field to a specific typed value.
///
/// Takes a generic type `T` that can be converted to/from reflection.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetFieldTyped<T>(
	cx: In<ActionContext<T>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<()>
where
	T: 'static + Send + Sync + Serialize + Typed,
{
	let field = fields.get(cx.id())?;
	query.set_field_typed(cx.id(), field, &cx.input)
}

/// An action that sets a field to a specific [`Value`], writing the document.
///
/// The document-backed twin of [`SetField`], see the [module docs](self).
/// Answers with the field's new value.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetDocField(
	cx: In<ActionContext<Value>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let entity = cx.id();
	let value = cx.take();
	query.with_field(entity, fields.get(entity)?, move |slot| {
		*slot = value;
		slot.clone()
	})
}

/// An action that appends a [`Value`] to a list-typed field.
///
/// Self-bound: appends to the entity's own [`Value`] list, coercing a missing or
/// null field into an empty list first. An append needs no schema check for the
/// same reason [`RemoveAtField`] does not, so unlike [`PushFieldTyped`] it stays
/// on the local `Value`, which is also what makes it authorable from markup:
/// BSX has no syntax for a type argument.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn PushField(
	cx: In<ActionContext<Value>>,
	mut values: Query<&mut Value>,
) -> Result<()> {
	values
		.get_mut(cx.id())?
		.as_list_mut_or_init()?
		.push(cx.input);
	Ok(())
}

impl PushField {
	/// Convenience constructor for a push against a field reference and path.
	pub fn bundle(field: FieldRef) -> impl Bundle {
		(field, PathPartial::new("push"), PushField)
	}
}

/// An action that appends a typed value to a list-typed field.
///
/// Coerces a missing or null field into an empty list first. When the document
/// carries a [`DocumentSchema`], the list's item type is checked against `T`,
/// which is the whole reason to reach for this over [`PushField`].
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn PushFieldTyped<T>(
	cx: In<ActionContext<T>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result
where
	T: 'static + Send + Sync + Serialize + Typed,
{
	let field = fields.get(cx.id())?;
	query.push_field(cx.id(), field, &cx.input)
}

/// An action that appends a [`Value`] to a list-typed field, writing the
/// document.
///
/// The document-backed twin of [`PushField`], see the [module docs](self).
/// Coerces a missing or null field into an empty list first and answers with
/// the field's new value, ie the whole list including the appended item.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn PushDocField(
	cx: In<ActionContext<Value>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let entity = cx.id();
	let value = cx.take();
	query.with_field(
		entity,
		fields.get(entity)?,
		move |slot| -> Result<Value> {
			slot.as_list_mut_or_init()?.push(value);
			Ok(slot.clone())
		},
	)?
}

/// An action that inserts a value at an index of a list-typed field.
///
/// The input is `(index, value)`; out-of-range indices are clamped to the list
/// length. Coerces a missing or null field into an empty list first.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn InsertAtField<T>(
	cx: In<ActionContext<(usize, T)>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result
where
	T: 'static + Send + Sync + Serialize + Typed + GetTypeRegistration,
{
	let entity = cx.id();
	let field = fields.get(entity)?;
	let (index, value) = cx.take();
	query.insert_at_field(entity, field, index, &value)
}

/// An action that removes the value at an index of a list-typed field,
/// returning the removed [`Value`] if the index was in bounds.
///
/// Self-bound: removes from the entity's own [`Value`] list. Removal needs no
/// schema check, so unlike [`InsertAtField`] it stays on the local `Value`.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn RemoveAtField(
	cx: In<ActionContext<usize>>,
	mut values: Query<&mut Value>,
) -> Result<Option<Value>> {
	let index = cx.input;
	let mut value = values.get_mut(cx.id())?;
	// error on a non-list field; an out-of-range index removes nothing. the
	// read-only length check avoids spuriously marking `Value` changed
	if index < value.as_list()?.len() {
		Ok(Some(value.as_list_mut()?.remove(index)))
	} else {
		Ok(None)
	}
}

/// An action that removes the value at an index of a list-typed field of the
/// document, returning the removed [`Value`] if the index was in bounds.
///
/// The document-backed twin of [`RemoveAtField`], see the [module docs](self).
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn RemoveAtDocField(
	cx: In<ActionContext<usize>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Option<Value>> {
	let entity = cx.id();
	let index = cx.input;
	query.with_field(
		entity,
		fields.get(entity)?,
		move |slot| -> Result<Option<Value>> {
			// error on a non-list field; an out-of-range index removes nothing
			let list = slot.as_list_mut()?;
			if index < list.len() {
				Ok(Some(list.remove(index)))
			} else {
				Ok(None)
			}
		},
	)?
}

/// An action that replaces the value at an index of a list-typed field.
///
/// The input is `(index, value)`; an out-of-range index errors rather than
/// growing the list, so a stale index never appends silently.
///
/// Self-bound: mutates the entity's own [`Value`] list. Like [`RemoveAtField`]
/// a replacement needs no schema check, which is also what makes it authorable
/// from markup: BSX has no syntax for a type argument.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetAtField(
	cx: In<ActionContext<(usize, Value)>>,
	mut values: Query<&mut Value>,
) -> Result {
	let entity = cx.id();
	let (index, new_value) = cx.take();
	*values
		.get_mut(entity)?
		.as_list_mut()?
		.get_mut(index)
		.ok_or_else(|| bevyhow!("no item at index {index}"))? = new_value;
	Ok(())
}

/// An action that replaces the value at an index of a list-typed field of the
/// document.
///
/// The document-backed twin of [`SetAtField`], see the [module docs](self). The
/// input is `(index, value)`; an out-of-range index errors rather than growing
/// the list. Answers with the field's new value.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetAtDocField(
	cx: In<ActionContext<(usize, Value)>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let entity = cx.id();
	let (index, new_value) = cx.take();
	query.with_field(
		entity,
		fields.get(entity)?,
		move |slot| -> Result<Value> {
			*slot
				.as_list_mut()?
				.get_mut(index)
				.ok_or_else(|| bevyhow!("no item at index {index}"))? = new_value;
			Ok(slot.clone())
		},
	)?
}

/// An action that reads the value at an index of a list-typed field.
///
/// Self-bound: reads the entity's own [`Value`] list, erroring when the index
/// is out of bounds.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadAtField(
	cx: In<ActionContext<usize>>,
	values: Query<&Value>,
) -> Result<Value> {
	let index = cx.input;
	values
		.get(cx.id())?
		.as_list()?
		.get(index)
		.cloned()
		.ok_or_else(|| bevyhow!("no item at index {index}"))
}

/// An action that reads the value at an index of a list-typed field of the
/// document.
///
/// The document-backed twin of [`ReadAtField`], see the [module docs](self).
/// Errors when the index is out of bounds.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadAtDocField(
	cx: In<ActionContext<usize>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let entity = cx.id();
	let index = cx.input;
	query
		.field_value(entity, fields.get(entity)?)?
		.as_list()?
		.get(index)
		.cloned()
		.ok_or_else(|| bevyhow!("no item at index {index}"))
}

/// An action that retrieves a field value.
///
/// Returns the [`Value`].
///
/// Self-bound: reads the entity's own [`Value`].
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadField(
	cx: In<ActionContext>,
	values: Query<&Value>,
) -> Result<Value> {
	values.get(cx.id())?.clone().xok()
}

/// An action that retrieves a field value from a document with type conversion.
///
/// Returns the value as a typed `T`.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadFieldTyped<T>(
	cx: In<ActionContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<T>
where
	T: 'static + Send + Sync + DeserializeOwned + Typed,
{
	let field = fields.get(cx.id())?;
	let doc = query.get(cx.id(), &field.document)?;
	doc.get_field::<T>(&field.field_path)?.xok()
}

/// An action that retrieves a field value from the document.
///
/// The document-backed twin of [`ReadField`], see the [module docs](self). A
/// field the document has never held answers with the [`FieldRef`] seed, ie
/// `[]` for a list, rather than erroring.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadDocField(
	cx: In<ActionContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let entity = cx.id();
	query.field_value(entity, fields.get(entity)?)
}

#[cfg(test)]
mod test {
	use super::*;
	#[cfg(feature = "template_serde")]
	use crate::prelude::DocumentUiPlugin;
	use beet_action::prelude::*;

	fn count_field() -> FieldRef { FieldRef::new("count") }

	#[beet_core::test]
	async fn increment_initializes_to_one() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(Increment::bundle(count_field())).id();

		world
			.entity_mut(entity)
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn increment_works_multiple_times() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(Increment::bundle(count_field())).id();

		world
			.entity_mut(entity)
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(1);

		world
			.entity_mut(entity)
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(2);

		world
			.entity_mut(entity)
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(3);
	}

	#[beet_core::test]
	async fn decrement_initializes_to_negative_one() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(Decrement::bundle(count_field())).id();

		world
			.entity_mut(entity)
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(-1);
	}

	#[beet_core::test]
	async fn decrement_works() {
		let mut world = AsyncPlugin::world();
		// the field seeds the entity's Value, which the action reads and mutates
		let entity = world
			.spawn(Decrement::bundle(count_field().with_init(Value::Int(5))))
			.id();

		world
			.entity_mut(entity)
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(4);
	}

	#[beet_core::test]
	async fn add_works() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(AddField::bundle(count_field().with_init(Value::Int(10))))
			.id();

		world
			.entity_mut(entity)
			.call::<i64, i64>(5)
			.await
			.unwrap()
			.xpect_eq(15);

		world
			.entity_mut(entity)
			.call::<i64, i64>(3)
			.await
			.unwrap()
			.xpect_eq(18);
	}

	/// Reads the local [`Value`] of `entity`.
	fn value_of(world: &World, entity: Entity) -> Value {
		world.entity(entity).get::<Value>().unwrap().clone()
	}

	#[beet_core::test]
	async fn set_field_creates_new_field() {
		let mut world = AsyncPlugin::world();
		// SetField writes the entity's Value; bidi sync carries it to the document
		let entity = world.spawn((FieldRef::new("message"), SetField)).id();

		world
			.entity_mut(entity)
			.call::<Value, ()>(value!("Hello"))
			.await
			.unwrap();

		value_of(&world, entity).xpect_eq(value!("Hello"));
	}

	#[beet_core::test]
	async fn set_field_updates_existing() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn((
				FieldRef::new("status").with_init(value!("pending")),
				SetField,
			))
			.id();

		world
			.entity_mut(entity)
			.call::<Value, ()>(value!("complete"))
			.await
			.unwrap();

		value_of(&world, entity).xpect_eq(value!("complete"));
	}

	#[beet_core::test]
	async fn set_field_typed_creates_new_field() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("message");
		let entity = world
			.spawn((field, SetFieldTyped::<String>::default()))
			.id();

		world
			.entity_mut(entity)
			.call::<String, ()>("Hello".to_string())
			.await
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldSegment::key("message")])
			.unwrap()
			.xpect_eq("Hello");
	}

	#[beet_core::test]
	async fn set_field_typed_updates_existing() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("status");
		let entity = world
			.spawn((
				Document::new(value!({ "status": "pending" })),
				field,
				SetFieldTyped::<String>::default(),
			))
			.id();

		world
			.entity_mut(entity)
			.call::<String, ()>("complete".to_string())
			.await
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldSegment::key("status")])
			.unwrap()
			.xpect_eq("complete");
	}

	#[beet_core::test]
	async fn get_field_retrieves_value() {
		let mut world = AsyncPlugin::world();
		// the field seeds the entity's Value, which ReadField returns
		let entity = world
			.spawn((FieldRef::new("data").with_init(Value::Int(42)), ReadField))
			.id();

		world
			.entity_mut(entity)
			.call::<(), Value>(())
			.await
			.unwrap()
			.xpect_eq(value!(42i64));
	}

	#[beet_core::test]
	async fn get_field_typed_retrieves_value() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("data");
		world
			.spawn((
				Document::new(value!({ "data": 42i64 })),
				field,
				ReadFieldTyped::<i64>::default(),
			))
			.call::<(), i64>(())
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[beet_core::test]
	async fn get_field_typed_nested() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new(vec!["user", "name"]);
		let entity = world
			.spawn((
				Document::new(value!({ "user": { "name": "Alice" } })),
				field,
				ReadFieldTyped::<String>::default(),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call::<(), String>(())
			.await
			.unwrap();

		result.xpect_eq("Alice");
	}

	#[beet_core::test]
	#[cfg(all(feature = "template_serde", feature = "json"))]
	fn roundtrip_increment_template() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init_plugin::<DocumentUiPlugin>();
		app.init();
		app.update();

		let entity =
			app.world_mut().spawn(Increment::bundle(count_field())).id();

		// Serialize
		let template_bytes = TemplateSaver::new()
			.with_entity_tree(app.world(), entity)
			.save(app.world(), MediaType::Json)
			.unwrap();
		template_bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("Increment");

		// Despawn original
		app.world_mut().entity_mut(entity).despawn();

		// Load
		let loaded = TemplateLoader::new(app.world_mut())
			.load(&template_bytes)
			.unwrap();
		app.update();

		// The loaded entity should have Increment and ActionMeta
		// (Action itself isn't serializable, but #[require] re-creates it)
		let loaded = *loaded.first().unwrap();
		app.world().entity(loaded).get::<Increment>().xpect_some();
		app.world().entity(loaded).get::<ActionMeta>().xpect_some();
	}

	fn todos_field() -> FieldRef {
		FieldRef::new("todos").with_init(Value::List(Vec::new()))
	}

	fn host_list(world: &World, host: Entity) -> Value {
		world
			.entity(host)
			.get::<Document>()
			.unwrap()
			.get_field_ref(&[FieldSegment::key("todos")])
			.unwrap()
			.clone()
	}

	/// The non-generic push is self-bound and schema-free, so it appends whatever
	/// [`Value`] it is handed, to the entity's own list, as [`RemoveAtField`]
	/// removes from it.
	#[beet_core::test]
	async fn push_value_appends() {
		let mut world = AsyncPlugin::world();
		let actor = world.spawn((todos_field(), PushField)).id();

		for name in ["ada", "bob"] {
			world
				.entity_mut(actor)
				.call::<Value, ()>(value!(name))
				.await
				.unwrap();
		}

		world
			.entity(actor)
			.get::<Value>()
			.unwrap()
			.xpect_eq(value!(["ada", "bob"]));
	}

	/// A field the document has never held is null, not a list, and an append
	/// wants the list either way.
	#[beet_core::test]
	async fn push_value_coerces_a_null_field() {
		let mut world = AsyncPlugin::world();
		let actor = world.spawn((FieldRef::new("todos"), PushField)).id();

		world
			.entity_mut(actor)
			.call::<Value, ()>(value!("ada"))
			.await
			.unwrap();

		world
			.entity(actor)
			.get::<Value>()
			.unwrap()
			.xpect_eq(value!(["ada"]));
	}

	#[beet_core::test]
	async fn push_appends() {
		let mut world = AsyncPlugin::world();
		let host = world.spawn(Document::default()).id();
		let actor = world
			.spawn((
				ChildOf(host),
				todos_field(),
				PushFieldTyped::<i32>::default(),
			))
			.id();

		world.entity_mut(actor).call::<i32, ()>(7).await.unwrap();
		world.entity_mut(actor).call::<i32, ()>(8).await.unwrap();

		host_list(&world, host).xpect_eq(value!([7i64, 8i64]));
	}

	/// Two actions over one field are two entities: an entity holds at most one
	/// action, so the push and the insert are siblings pointed at the same
	/// [`FieldRef`] rather than colocated.
	#[beet_core::test]
	async fn push_and_insert() {
		let mut world = AsyncPlugin::world();
		let host = world.spawn(Document::default()).id();
		let push = world
			.spawn((
				ChildOf(host),
				todos_field(),
				PushFieldTyped::<i32>::default(),
			))
			.id();
		let insert = world
			.spawn((
				ChildOf(host),
				todos_field(),
				InsertAtField::<i32>::default(),
			))
			.id();

		for value in [1i32, 2, 3] {
			world.entity_mut(push).call::<i32, ()>(value).await.unwrap();
		}
		// list is now [1, 2, 3]
		world
			.entity_mut(insert)
			.call::<(usize, i32), ()>((1, 99))
			.await
			.unwrap();

		host_list(&world, host).xpect_eq(value!([1i64, 99i64, 2i64, 3i64]));
	}

	/// The `*DocField` contract: the write lands in the document itself, so a
	/// sibling read sees it with no sync frame in between, and the write answers
	/// with the field's new value rather than an empty `null`.
	#[beet_core::test]
	async fn doc_field_write_is_visible_to_the_next_read() {
		let mut world = AsyncPlugin::world();
		let host = world.spawn(Document::default()).id();
		let push = world
			.spawn((ChildOf(host), todos_field(), PushDocField))
			.id();
		let read = world
			.spawn((ChildOf(host), todos_field(), ReadDocField))
			.id();

		world
			.entity_mut(push)
			.call::<Value, Value>(value!("ada"))
			.await
			.unwrap()
			.xpect_eq(value!(["ada"]));
		world
			.entity_mut(read)
			.call::<(), Value>(())
			.await
			.unwrap()
			.xpect_eq(value!(["ada"]));
	}

	/// A document that has never held the field answers with the [`FieldRef`]
	/// seed, so an untouched list reads as `[]` rather than erroring.
	#[beet_core::test]
	async fn read_doc_field_falls_back_to_the_seed() {
		let mut world = AsyncPlugin::world();
		let host = world.spawn(Document::default()).id();
		world
			.spawn((ChildOf(host), todos_field(), ReadDocField))
			.call::<(), Value>(())
			.await
			.unwrap()
			.xpect_eq(Value::List(Vec::new()));
	}

	#[beet_core::test]
	async fn remove_at_value() {
		let mut world = AsyncPlugin::world();
		// seed the actor's local list, as bidi sync would in a running app
		let actor = world
			.spawn((
				FieldRef::new("todos")
					.with_init(Value::new_list([1i64, 99, 2, 3])),
				RemoveAtField,
			))
			.id();

		// removing the head returns it and leaves the tail behind
		world
			.entity_mut(actor)
			.call::<usize, Option<Value>>(0)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(value!(1i64));
		value_of(&world, actor).xpect_eq(value!([99i64, 2i64, 3i64]));

		// an out-of-range index removes nothing
		world
			.entity_mut(actor)
			.call::<usize, Option<Value>>(10)
			.await
			.unwrap()
			.xpect_none();
	}

	#[beet_core::test]
	async fn remove_at_rejects_non_list() {
		let mut world = AsyncPlugin::world();
		let actor = world
			.spawn((
				FieldRef::new("todos").with_init(value!("not a list")),
				RemoveAtField,
			))
			.id();

		world
			.entity_mut(actor)
			.call::<usize, Option<Value>>(0)
			.await
			.is_err()
			.xpect_true();
	}

	#[beet_core::test]
	async fn push_rejects_wrong_type() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct TodoDoc {
			todos: Vec<String>,
		}

		let mut world = AsyncPlugin::world();
		let host = world
			.spawn((Document::default(), DocumentSchema::of::<TodoDoc>()))
			.id();
		let actor = world
			.spawn((
				ChildOf(host),
				todos_field(),
				PushFieldTyped::<i64>::default(),
			))
			.id();

		world
			.entity_mut(actor)
			.call::<i64, ()>(7)
			.await
			.is_err()
			.xpect_true();
	}

	#[beet_core::test]
	async fn set_field_typed_rejects_wrong_type() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct CountDoc {
			count: i64,
		}

		let mut world = AsyncPlugin::world();
		let host = world
			.spawn((Document::default(), DocumentSchema::of::<CountDoc>()))
			.id();
		let actor = world
			.spawn((
				ChildOf(host),
				FieldRef::new("count"),
				SetFieldTyped::<String>::default(),
			))
			.id();

		world
			.entity_mut(actor)
			.call::<String, ()>("oops".to_string())
			.await
			.is_err()
			.xpect_true();
	}
}
