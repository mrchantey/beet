use crate::prelude::*;
use beet_core::prelude::*;



/// A tool that increments a numeric field in a document, returning the new value.
///
/// When triggered, this tool:
/// 1. Reads the current value from the specified field
/// 2. Increments it by 1
/// 3. Writes the new value back
/// 4. Returns the new value
///
/// If the field doesn't exist, it will be initialized to 1.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
/// let field = FieldRef::new(DocumentPath::Card, "counter");
/// let entity = world.spawn(increment(field)).id();
///
/// // First call initializes to 1
/// let result = world.entity_mut(entity).send_blocking::<(), i64>(()).unwrap();
/// assert_eq!(result, 1);
///
/// // Second call increments to 2
/// let result = world.entity_mut(entity).send_blocking::<(), i64>(()).unwrap();
/// assert_eq!(result, 2);
/// ```
pub fn increment(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<i64> {
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					let current = value.as_i64().unwrap_or(0);
					let new_value = current + 1;
					*value = serde_json::json!(new_value);
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
/// If the field doesn't exist, it will be initialized to -1.
pub fn decrement(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<i64> {
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					let current = value.as_i64().unwrap_or(0);
					let new_value = current - 1;
					*value = serde_json::json!(new_value);
					new_value
				})
			},
		),
	)
}

/// A tool that adds a value to a numeric field in a document.
///
/// Takes the amount to add as input and returns the new value.
pub fn add(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext<i64>>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<i64> {
				let amount = cx.payload;
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					let current = value.as_i64().unwrap_or(0);
					let new_value = current + amount;
					*value = serde_json::json!(new_value);
					new_value
				})
			},
		),
	)
}

/// A tool that sets a field to a specific JSON value.
///
/// Takes a `serde_json::Value` as input and stores it in the specified field.
pub fn set_field(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext<String>>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<()> {
				let new_value: serde_json::Value =
					serde_json::from_str(&cx.payload).map_err(|err| {
						bevyhow!("Failed to parse JSON: {err}")
					})?;
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					*value = new_value.clone();
				})
			},
		),
	)
}

/// A tool that sets a field to a specific value, with type serialization.
///
/// Takes a JSON string as input, deserializes it, and stores it in the specified field.
pub fn set_field_typed(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext<String>>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<()> {
				let new_value: serde_json::Value =
					serde_json::from_str(&cx.payload).map_err(|err| {
						bevyhow!("Failed to parse JSON: {err}")
					})?;
				let field = fields.get(cx.tool)?;
				query.with_field(cx.tool, field, |value| {
					*value = new_value.clone();
				})
			},
		),
	)
}

/// A tool that retrieves a field value from a document.
///
/// Returns the `serde_json::Value` directly.
pub fn get_field(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<String> {
				let field = fields.get(cx.tool)?;
				let doc = query.get(cx.tool, &field.document)?;
				let value = doc.get_field_ref(&field.field_path)?;
				serde_json::to_string(value)
					.map_err(|err| bevyhow!("Failed to serialize JSON: {err}"))
			},
		),
	)
}

/// A tool that retrieves a field value from a document with type serialization.
///
/// Returns the JSON value as a string.
pub fn get_field_typed(field: FieldRef) -> impl Bundle {
	(
		field,
		tool(
			|cx: In<ToolContext>,
			 mut query: DocumentQuery,
			 fields: Query<&FieldRef>|
			 -> Result<String> {
				let field = fields.get(cx.tool)?;
				let doc = query.get(cx.tool, &field.document)?;
				let value = doc.get_field_ref(&field.field_path)?;
				serde_json::to_string(value)
					.map_err(|err| bevyhow!("Failed to serialize JSON: {err}"))
			},
		),
	)
}



#[cfg(test)]
mod test {
	use super::*;

	fn count_field() -> FieldRef { FieldRef::new(DocumentPath::Card, "count") }

	#[test]
	fn increment_initializes_to_one() {
		let mut world = World::new();
		let entity = world.spawn((Card, increment(count_field()))).id();

		world
			.entity_mut(entity)
			.send_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(1);
	}

	#[test]
	fn increment_works_multiple_times() {
		let mut world = World::new();
		let entity = world.spawn((Card, increment(count_field()))).id();

		world
			.entity_mut(entity)
			.send_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(1);

		world
			.entity_mut(entity)
			.send_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(2);

		world
			.entity_mut(entity)
			.send_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(3);
	}

	#[test]
	fn decrement_initializes_to_negative_one() {
		let mut world = World::new();
		let entity = world.spawn((Card, decrement(count_field()))).id();

		world
			.entity_mut(entity)
			.send_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(-1);
	}

	#[test]
	fn decrement_works() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"count": 5})),
				decrement(count_field()),
			))
			.id();

		world
			.entity_mut(entity)
			.send_blocking::<(), i64>(())
			.unwrap()
			.xpect_eq(4);
	}

	#[test]
	fn add_works() {
		let mut world = World::new();
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"count": 10})),
				add(count_field()),
			))
			.id();

		world
			.entity_mut(entity)
			.send_blocking::<i64, i64>(5)
			.unwrap()
			.xpect_eq(15);

		world
			.entity_mut(entity)
			.send_blocking::<i64, i64>(3)
			.unwrap()
			.xpect_eq(18);
	}

	#[test]
	fn set_field_creates_new_field() {
		let mut world = World::new();
		let field = FieldRef::new(DocumentPath::Card, "message");
		let entity = world.spawn((Card, set_field(field))).id();

		world
			.entity_mut(entity)
			.send_blocking::<String, ()>(r#""Hello""#.to_string())
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
		let field = FieldRef::new(DocumentPath::Card, "status");
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"status": "pending"})),
				set_field(field),
			))
			.id();

		world
			.entity_mut(entity)
			.send_blocking::<String, ()>(r#""complete""#.to_string())
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
		let field = FieldRef::new(DocumentPath::Card, "message");
		let entity = world.spawn((Card, set_field_typed(field))).id();

		world
			.entity_mut(entity)
			.send_blocking::<String, ()>(r#""Hello""#.to_string())
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
		let field = FieldRef::new(DocumentPath::Card, "status");
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"status": "pending"})),
				set_field_typed(field),
			))
			.id();

		world
			.entity_mut(entity)
			.send_blocking::<String, ()>(r#""complete""#.to_string())
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
		let field = FieldRef::new(DocumentPath::Card, "data");
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"data": 42})),
				get_field(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.send_blocking::<(), String>(())
			.unwrap();

		result.xpect_eq("42");
	}

	#[test]
	fn get_field_nested() {
		let mut world = World::new();
		let field = FieldRef::new(
			DocumentPath::Card,
			vec!["user", "name"].into_field_path_vec(),
		);
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"user": {"name": "Alice"}})),
				get_field(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.send_blocking::<(), String>(())
			.unwrap();

		result.xpect_eq(r#""Alice""#);
	}

	#[test]
	fn get_field_typed_retrieves_value() {
		let mut world = World::new();
		let field = FieldRef::new(DocumentPath::Card, "data");
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"data": 42})),
				get_field_typed(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.send_blocking::<(), String>(())
			.unwrap();

		result.xpect_eq("42");
	}

	#[test]
	fn get_field_typed_nested() {
		let mut world = World::new();
		let field = FieldRef::new(
			DocumentPath::Card,
			vec!["user", "name"].into_field_path_vec(),
		);
		let entity = world
			.spawn((
				Card,
				Document::new(serde_json::json!({"user": {"name": "Alice"}})),
				get_field_typed(field),
			))
			.id();

		let result = world
			.entity_mut(entity)
			.send_blocking::<(), String>(())
			.unwrap();

		result.xpect_eq(r#""Alice""#);
	}
}
