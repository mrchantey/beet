use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::reflect::Typed;

/// A tool that increments a numeric field in a document, returning the new value.
///
/// When triggered, this tool:
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
/// use beet_node::prelude::*;
///
/// let mut world = AsyncPlugin::world();
/// let field = FieldRef::new("counter");
/// let entity = world.spawn(increment(field)).id();
/// ```
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Increment(
	cx: In<ToolContext>,
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

/// A tool that decrements a numeric field in a document, returning the new value.
///
/// When triggered, this tool:
/// 1. Reads the current value from the specified field
/// 2. Decrements it by 1
/// 3. Writes the new value back
/// 4. Returns the new value
///
/// If the field doesn't exist or is not an i64, it will be initialized to -1.
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Decrement(
	cx: In<ToolContext>,
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

/// A tool that adds a value to a numeric field in a document.
///
/// Takes the amount to add as input and returns the new value.
/// If the field doesn't exist or is not an i64, it will be initialized to the provided value.
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn AddField(
	cx: In<ToolContext<i64>>,
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

/// A tool that sets a field to a specific [`Value`].
///
/// Takes a [`Value`] as input and stores it in the specified field.
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetField(
	cx: In<ToolContext<Value>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<()> {
	let field = fields.get(cx.id())?;
	query.with_field(cx.id(), field, move |value| {
		*value = cx.input;
	})
}

/// Convenience constructor for set_field with a field reference and path.
pub fn set_field(field: FieldRef) -> impl Bundle {
	(field, PathPartial::new("set-field"), SetField)
}

/// A tool that sets a field to a specific typed value.
///
/// Takes a generic type `T` that can be converted to/from reflection.
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn SetFieldTyped<T>(
	cx: In<ToolContext<T>>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<()>
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	let field = fields.get(cx.id())?;
	let new_value = Value::from_reflect(&cx.input)?;
	query.with_field(cx.id(), field, move |value| {
		*value = new_value;
	})
}

/// Convenience constructor for set_field_typed with a field reference and path.
pub fn set_field_typed<T>(field: FieldRef) -> impl Bundle
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	(
		field,
		PathPartial::new("set-field-typed"),
		SetFieldTyped::<T>(core::marker::PhantomData),
	)
}

/// A tool that retrieves a field value from a document.
///
/// Returns the [`Value`].
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadField(
	cx: In<ToolContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<Value> {
	let field = fields.get(cx.id())?;
	let doc = query.get(cx.id(), &field.document)?;
	doc.get_field_ref(&field.field_path)
		.map(|val| val.clone())?
		.xok()
}

/// Convenience constructor for get_field with a field reference and path.
pub fn get_field(field: FieldRef) -> impl Bundle {
	(field, PathPartial::new("get-field"), ReadField)
}

/// A tool that retrieves a field value from a document with type conversion.
///
/// Returns the value as a typed `T`.
#[tool]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn ReadFieldTyped<T>(
	cx: In<ToolContext>,
	mut query: DocumentQuery,
	fields: Query<&FieldRef>,
) -> Result<T>
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	let field = fields.get(cx.id())?;
	let doc = query.get(cx.id(), &field.document)?;
	doc.get_field::<T>(&field.field_path)?.xok()
}

/// Convenience constructor for get_field_typed with a field reference and path.
pub fn get_field_typed<T>(field: FieldRef) -> impl Bundle
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	(
		field,
		PathPartial::new("get-field-typed"),
		ReadFieldTyped::<T>(core::marker::PhantomData),
	)
}



#[cfg(test)]
mod test {
	use super::*;
	use beet_tool::prelude::*;
	use bevy::ecs::entity::EntityHashMap;

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
		let entity = world.spawn(set_field(field)).id();

		world
			.entity_mut(entity)
			.call::<Value, ()>(val!("Hello"))
			.await
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldPath::ObjectKey("message".to_string())])
			.unwrap()
			.xpect_eq("Hello");
	}

	#[beet_core::test]
	async fn set_field_updates_existing() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("status");
		let entity = world
			.spawn((
				Document::new(val!({ "status": "pending" })),
				set_field(field),
			))
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
			.get_field::<String>(&[FieldPath::ObjectKey("status".to_string())])
			.unwrap()
			.xpect_eq("complete");
	}

	#[beet_core::test]
	async fn set_field_typed_creates_new_field() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("message");
		let entity = world.spawn(set_field_typed::<String>(field)).id();

		world
			.entity_mut(entity)
			.call::<String, ()>("Hello".to_string())
			.await
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldPath::ObjectKey("message".to_string())])
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
				set_field_typed::<String>(field),
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
			.get_field::<String>(&[FieldPath::ObjectKey("status".to_string())])
			.unwrap()
			.xpect_eq("complete");
	}

	#[beet_core::test]
	async fn get_field_retrieves_value() {
		let mut world = AsyncPlugin::world();
		let field = FieldRef::new("data");
		let entity = world
			.spawn((Document::new(val!({ "data": 42i64 })), get_field(field)))
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
				get_field(field),
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
				get_field_typed::<i64>(field),
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
				get_field_typed::<String>(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call::<(), String>(())
			.await
			.unwrap();

		result.xpect_eq("Alice");
	}

	#[test]
	fn roundtrip_increment_scene() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init_plugin::<DocumentPlugin>();
		app.init();
		app.update();

		let entity = app.world_mut().spawn(increment(count_field())).id();

		// Serialize
		let scene = SceneSaver::new(app.world_mut())
			.with_entity_tree(entity)
			.save_ron()
			.unwrap();
		scene.xref().xpect_contains("Increment");

		// Despawn original
		app.world_mut().entity_mut(entity).despawn();

		// Load
		let mut entity_map = EntityHashMap::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load_ron(&scene)
			.unwrap();
		app.update();

		// The loaded entity should have Increment and ToolMeta
		// (Tool itself isn't serializable, but #[require] re-creates it)
		let loaded = *entity_map.values().next().unwrap();
		app.world().entity(loaded).get::<Increment>().xpect_some();
		app.world().entity(loaded).get::<ToolMeta>().xpect_some();
	}
}
