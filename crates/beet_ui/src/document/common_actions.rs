use crate::prelude::*;
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
/// # Example
///
/// ```no_run
/// use beet_core::prelude::*;
/// use beet_ui::prelude::*;
///
/// let mut world = AsyncPlugin::world();
/// let field = FieldRef::new("counter");
/// let entity = world.spawn(increment(field)).id();
/// ```
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Increment(
	cx: In<ActionContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<i64> {
	let field = fields.get(cx.id())?;
	query.with_field(cx.id(), field, |value| {
		let current = value.as_i64().unwrap_or(0);
		let new_value = current + 1;
		*value = Value::Int(new_value);
		new_value
	})
}

/// Convenience constructor for increment with a field reference and path.
pub fn increment(field: FieldRef) -> impl Bundle {
	(field, PathPartial::new("increment"), Increment)
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
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Decrement(
	cx: In<ActionContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<i64> {
	let field = fields.get(cx.id())?;
	query.with_field(cx.id(), field, |value| {
		let current = value.as_i64().unwrap_or(0);
		let new_value = current - 1;
		*value = Value::Int(new_value);
		new_value
	})
}

/// Convenience constructor for decrement with a field reference and path.
pub fn decrement(field: FieldRef) -> impl Bundle {
	(field, PathPartial::new("decrement"), Decrement)
}

/// An action that adds a value to a numeric field in a document.
///
/// Takes the amount to add as input and returns the new value.
/// If the field doesn't exist or is not an i64, it will be initialized to the provided value.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn AddField(
	cx: In<ActionContext<i64>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<i64> {
	let field = fields.get(cx.id())?;
	query.with_field(cx.id(), field, |value| {
		let current = value.as_i64().unwrap_or(0);
		let new_value = current + cx.input;
		*value = Value::Int(new_value);
		new_value
	})
}

/// Convenience constructor for add with a field reference and path.
pub fn add(field: FieldRef) -> impl Bundle {
	(field, PathPartial::new("add"), AddField)
}

/// An action that sets a field to a specific [`Value`].
///
/// Takes a [`Value`] as input and stores it in the specified field.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetField(
	cx: In<ActionContext<Value>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<()> {
	let field = fields.get(cx.id())?;
	query.with_field(cx.id(), field, move |value| {
		*value = cx.input;
	})
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

/// An action that appends a value to a list-typed field.
///
/// Coerces a missing or null field into an empty list first. When the document
/// carries a [`DocumentSchema`], the list's item type is checked against `T`.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn PushField<T>(
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
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn RemoveAtField(
	cx: In<ActionContext<usize>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Option<Value>> {
	let field = fields.get(cx.id())?;
	query.remove_at_field(cx.id(), field, cx.input)
}

/// An action that retrieves a field value from a document.
///
/// Returns the [`Value`].
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadField(
	cx: In<ActionContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let field = fields.get(cx.id())?;
	let doc = query.get(cx.id(), &field.document)?;
	doc.get_field_ref(&field.field_path)
		.map(|val| val.clone())?
		.xok()
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


#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	fn count_field() -> FieldRef { FieldRef::new("count") }

	#[beet_core::test]
	async fn increment_initializes_to_one() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(increment(count_field())).id();

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
		let entity = world.spawn(increment(count_field())).id();

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
		let entity = world.spawn(decrement(count_field())).id();

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
		let entity = world
			.spawn((
				Document::new(val!({ "count": 5i64 })),
				decrement(count_field()),
			))
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
			.spawn((
				Document::new(val!({ "count": 10i64 })),
				add(count_field()),
			))
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

	#[beet_core::test]
	async fn set_field_creates_new_field() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("message");
		let entity = world.spawn((field, SetField)).id();

		world
			.entity_mut(entity)
			.call::<Value, ()>(val!("Hello"))
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
	async fn set_field_updates_existing() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("status");
		let entity = world
			.spawn((Document::new(val!({ "status": "pending" })), field, SetField))
			.id();

		world
			.entity_mut(entity)
			.call::<Value, ()>(val!("complete"))
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
	async fn set_field_typed_creates_new_field() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("message");
		let entity =
			world.spawn((field, SetFieldTyped::<String>::default())).id();

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
				Document::new(val!({ "status": "pending" })),
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
		let field = FieldRef::new("data");
		let entity = world
			.spawn((Document::new(val!({ "data": 42i64 })), field, ReadField))
			.id();

		let result = world
			.entity_mut(entity)
			.call::<(), Value>(())
			.await
			.unwrap();

		result.xpect_eq(val!(42i64));
	}

	#[beet_core::test]
	async fn get_field_nested() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new(vec!["user", "name"]);
		let entity = world
			.spawn((
				Document::new(val!({ "user": { "name": "Alice" } })),
				field,
				ReadField,
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call::<(), Value>(())
			.await
			.unwrap();

		result.xpect_eq(val!("Alice"));
	}

	#[beet_core::test]
	async fn get_field_typed_retrieves_value() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("data");
		world
			.spawn((
				Document::new(val!({ "data": 42i64 })),
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
				Document::new(val!({ "user": { "name": "Alice" } })),
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
	#[cfg(feature = "bevy_scene")]
	fn roundtrip_increment_scene() {
		use bevy::ecs::entity::EntityHashMap;
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init_plugin::<DocumentPlugin>();
		app.init();
		app.update();

		let entity = app.world_mut().spawn(increment(count_field())).id();

		// Serialize
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entity_tree(entity)
			.save(MediaType::Ron)
			.unwrap();
		scene_bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("Increment");

		// Despawn original
		app.world_mut().entity_mut(entity).despawn();

		// Load
		let mut entity_map = EntityHashMap::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load(&scene_bytes)
			.unwrap();
		app.update();

		// The loaded entity should have Increment and ActionMeta
		// (Action itself isn't serializable, but #[require] re-creates it)
		let loaded = *entity_map.values().next().unwrap();
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

	#[beet_core::test]
	async fn push_appends() {
		let mut world = AsyncPlugin::world();
		let host = world.spawn(Document::default()).id();
		let actor = world
			.spawn((ChildOf(host), todos_field(), PushField::<i32>::default()))
			.id();

		world.entity_mut(actor).call::<i32, ()>(7).await.unwrap();
		world.entity_mut(actor).call::<i32, ()>(8).await.unwrap();

		host_list(&world, host).xpect_eq(val!([7i64, 8i64]));
	}

	#[beet_core::test]
	async fn insert_and_remove() {
		let mut world = AsyncPlugin::world();
		let host = world.spawn(Document::default()).id();
		let actor = world
			.spawn((
				ChildOf(host),
				todos_field(),
				PushField::<i32>::default(),
				InsertAtField::<i32>::default(),
				RemoveAtField,
			))
			.id();

		for value in [1i32, 2, 3] {
			world.entity_mut(actor).call::<i32, ()>(value).await.unwrap();
		}
		// list is now [1, 2, 3]
		world
			.entity_mut(actor)
			.call::<(usize, i32), ()>((1, 99))
			.await
			.unwrap();
		// list is now [1, 99, 2, 3]
		world
			.entity_mut(actor)
			.call::<usize, Option<Value>>(0)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(val!(1i64));

		host_list(&world, host).xpect_eq(val!([99i64, 2i64, 3i64]));
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
			.spawn((ChildOf(host), todos_field(), PushField::<i64>::default()))
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
