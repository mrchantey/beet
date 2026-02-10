use crate::prelude::*;
use beet_core::prelude::*;
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
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
/// let field = FieldRef::new("counter");
/// let entity = world.spawn(increment(field)).id();
///
/// // First call initializes to 1
/// let result = world.entity_mut(entity).call_blocking::<(), i64>(()).unwrap();
/// assert_eq!(result, 1);
///
/// // Second call increments to 2
/// let result = world.entity_mut(entity).call_blocking::<(), i64>(()).unwrap();
/// assert_eq!(result, 2);
/// ```
pub fn increment(field: FieldRef) -> impl Bundle {
	(
		field,
		PathPartial::new("increment"),
		tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<i64> {
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					let current = value.as_i64().unwrap_or(0);
					let new_value = current + 1;
					*value = Value::I64(new_value);
					new_value
				})
			},
		),
	)
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
pub fn decrement(field: FieldRef) -> impl Bundle {
	(
		field,
		PathPartial::new("decrement"),
		tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<i64> {
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					let current = value.as_i64().unwrap_or(0);
					let new_value = current - 1;
					*value = Value::I64(new_value);
					new_value
				})
			},
		),
	)
}

/// A tool that adds a value to a numeric field in a document.
///
/// Takes the amount to add as input and returns the new value.
/// If the field doesn't exist or is not an i64, it will be initialized to the provided value.
pub fn add(field: FieldRef) -> impl Bundle {
	(
		field,
		PathPartial::new("add"),
		tool(
			|In(ToolContext { tool, input }): In<ToolContext<i64>>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<i64> {
				let field = fields.get(tool)?;
				query.with_field(tool, field, |value| {
					let current = value.as_i64().unwrap_or(0);
					let new_value = current + input;
					*value = Value::I64(new_value);
					new_value
				})
			},
		),
	)
}

/// A tool that sets a field to a specific [`Value`].
///
/// Takes a [`Value`] as input and stores it in the specified field.
pub fn set_field(field: FieldRef) -> impl Bundle {
	(
		field,
		PathPartial::new("set-field"),
		direct_tool(
			|In(cx): In<ToolContext<Value>>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<()> {
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, move |value| {
					*value = cx.input;
				})
			},
		),
	)
}

/// A tool that sets a field to a specific typed value.
///
/// Takes a generic type `T` that can be converted to/from reflection.
pub fn set_field_typed<T>(field: FieldRef) -> impl Bundle
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	(
		field,
		PathPartial::new("set-field-typed"),
		direct_tool(
			move |cx: In<ToolContext<T>>,
			      mut query: DocumentQuery,
			      fields: Query<&FieldRef>|
			      -> Result<()> {
				let field = fields.get(cx.tool)?;
				let new_value = Value::from_reflect(&cx.input)?;
				query.with_field(cx.tool, field, move |value| {
					*value = new_value;
				})
			},
		),
	)
}

/// A tool that retrieves a field value from a document.
///
/// Returns the [`Value`].
pub fn get_field(field: FieldRef) -> impl Bundle {
	(
		field,
		PathPartial::new("get-field"),
		direct_tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<Value> {
				let field = fields.get(cx.tool)?;
				let doc = query.get(cx.tool, &field.document)?;
				doc.get_field_ref(&field.field_path)
					.map(|v| v.clone())?
					.xok()
			},
		),
	)
}

/// A tool that retrieves a field value from a document with type conversion.
///
/// Returns the value as a typed `T`.
pub fn get_field_typed<T>(field: FieldRef) -> impl Bundle
where
	T: 'static + Send + Sync + FromReflect + Typed,
{
	(
		field,
		PathPartial::new("get-field-typed"),
		direct_tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<T> {
				let field = fields.get(cx.tool)?;
				let doc = query.get(cx.tool, &field.document)?;
				doc.get_field::<T>(&field.field_path)?.xok()
			},
		),
	)
}



#[cfg(test)]
mod test {
	use super::*;

	fn count_field() -> FieldRef { FieldRef::new("count") }

	#[test]
	fn increment_initializes_to_one() {
		let mut world = World::new();
		let entity = world.spawn((Card, increment(count_field()))).id();

		world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(1);
	}

	#[test]
	fn increment_works_multiple_times() {
		let mut world = World::new();
		let entity = world.spawn((Card, increment(count_field()))).id();

		world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(1);

		world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(2);

		world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(3);
	}

	#[test]
	fn decrement_initializes_to_negative_one() {
		let mut world = World::new();
		let entity = world.spawn((Card, decrement(count_field()))).id();

		world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(-1);
	}

	#[test]
	fn decrement_works() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "count": 5i64 })),
				decrement(count_field()),
			))
			.id();

		world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(4);
	}

	#[test]
	fn add_works() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "count": 10i64 })),
				add(count_field()),
			))
			.id();

		world
			.entity_mut(entity)
			.call_blocking::<i64, i64>(5)
			.unwrap()
			.xpect_eq(15);

		world
			.entity_mut(entity)
			.call_blocking::<i64, i64>(3)
			.unwrap()
			.xpect_eq(18);
	}

	#[test]
	fn set_field_creates_new_field() {
		let mut world = World::new();
		let field = FieldRef::new("message");
		let entity = world.spawn((Card, set_field(field))).id();

		world
			.entity_mut(entity)
			.call_blocking::<Value, ()>(val!("Hello"))
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldPath::ObjectKey("message".to_string())])
			.unwrap()
			.xpect_eq("Hello");
	}

	#[test]
	fn set_field_updates_existing() {
		let mut world = World::new();
		let field = FieldRef::new("status");
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "status": "pending" })),
				set_field(field),
			))
			.id();

		world
			.entity_mut(entity)
			.call_blocking::<Value, ()>(val!("complete"))
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldPath::ObjectKey("status".to_string())])
			.unwrap()
			.xpect_eq("complete");
	}

	#[test]
	fn set_field_typed_creates_new_field() {
		let mut world = World::new();
		let field = FieldRef::new("message");
		let entity = world.spawn((Card, set_field_typed::<String>(field))).id();

		world
			.entity_mut(entity)
			.call_blocking::<String, ()>("Hello".to_string())
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldPath::ObjectKey("message".to_string())])
			.unwrap()
			.xpect_eq("Hello");
	}

	#[test]
	fn set_field_typed_updates_existing() {
		let mut world = World::new();
		let field = FieldRef::new("status");
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "status": "pending" })),
				set_field_typed::<String>(field),
			))
			.id();

		world
			.entity_mut(entity)
			.call_blocking::<String, ()>("complete".to_string())
			.unwrap();

		world
			.entity(entity)
			.get::<Document>()
			.unwrap()
			.get_field::<String>(&[FieldPath::ObjectKey("status".to_string())])
			.unwrap()
			.xpect_eq("complete");
	}

	#[test]
	fn get_field_retrieves_value() {
		let mut world = World::new();
		let field = FieldRef::new("data");
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "data": 42i64 })),
				get_field(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call_blocking::<(), Value>(())
			.unwrap();

		result.xpect_eq(val!(42i64));
	}

	#[test]
	fn get_field_nested() {
		let mut world = World::new();
		let field = FieldRef::new(vec!["user", "name"]);
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "user": { "name": "Alice" } })),
				get_field(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call_blocking::<(), Value>(())
			.unwrap();

		result.xpect_eq(val!("Alice"));
	}

	#[test]
	fn get_field_typed_retrieves_value() {
		let mut world = World::new();
		let field = FieldRef::new("data");
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "data": 42i64 })),
				get_field_typed::<i64>(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call_blocking::<(), i64>(())
			.unwrap();

		result.xpect_eq(42);
	}

	#[test]
	fn get_field_typed_nested() {
		let mut world = World::new();
		let field = FieldRef::new(vec!["user", "name"]);
		let entity = world
			.spawn((
				Card,
				Document::new(val!({ "user": { "name": "Alice" } })),
				get_field_typed::<String>(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.call_blocking::<(), String>(())
			.unwrap();

		result.xpect_eq("Alice");
	}
}
