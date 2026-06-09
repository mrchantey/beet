use crate::prelude::*;
use bevy::reflect::Typed;

/// System parameter for working with documents.
///
/// Provides convenient methods for accessing and modifying documents
/// on entities, with automatic entity resolution based on [`DocumentPath`].
#[derive(SystemParam)]
pub struct DocumentQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	doc_query: Query<'w, 's, &'static mut Document>,
	schemas: Query<'w, 's, &'static DocumentSchema>,
	/// Shared upward resolver for the [`DocumentScope`] prefix, so reads and
	/// writes scope through the same walk.
	scopes: ScopeQuery<'w, 's>,
	commands: Commands<'w, 's>,
}

impl<'w, 's> DocumentQuery<'w, 's> {
	/// Resolve a [`DocumentPath`] to the actual entity that owns the document.
	pub fn entity(&mut self, subject: Entity, path: &DocumentPath) -> Entity {
		match path {
			DocumentPath::Root => self.ancestors.root_ancestor(subject),
			DocumentPath::Ancestor => self
				.ancestors
				.iter_ancestors_inclusive(subject)
				.find(|entity| self.doc_query.contains(*entity))
				.unwrap_or_else(|| self.ancestors.root_ancestor(subject)),
			DocumentPath::Entity(entity) => *entity,
			DocumentPath::This => subject,
		}
	}

	/// Returns the query item for the document.
	pub fn get(
		&mut self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<&Document> {
		let doc_entity = self.entity(entity, path);
		self.doc_query.get(doc_entity)?.xok()
	}


	/// Returns the mutable query item for the document.
	pub fn get_mut(
		&mut self,
		subject: Entity,
		path: &DocumentPath,
	) -> Result<Mut<'_, Document>> {
		let doc_entity = self.entity(subject, path);
		self.doc_query.get_mut(doc_entity)?.xok()
	}

	/// Execute a function with a mutable reference to a field.
	///
	/// If the document or field doesn't exist and [`FieldRef::on_missing`] is set to initialize,
	/// they will be initialized with the specified value. Otherwise an error is returned.
	pub fn with_field<Out>(
		&mut self,
		subject: Entity,
		field: &FieldRef,
		func: impl FnOnce(&mut Value) -> Out,
	) -> Result<Out> {
		let doc_entity = self.entity(subject, &field.document);
		// resolve the scope prefix fresh, so writes are reactive by construction
		let field_path = self.scopes.resolved_path(subject, &field.field_path);

		if let Ok(mut doc) = self.doc_query.get_mut(doc_entity) {
			let value = if let Ok(value) = doc.get_field_mut(&field_path) {
				value
			} else if let OnMissingField::Init { value: init_value } =
				&field.on_missing
			{
				doc.insert(&field_path, init_value)?
			} else {
				return Err(DocumentError::ObjectKeyNotFound {
					path: field_path.clone(),
					key: format!("{:?}", field_path),
				}
				.into());
			};
			Ok(func(value))
		} else if let OnMissingField::Init { value: init_value } =
			&field.on_missing
		{
			// create the document and run the method with it
			let mut doc = Document::default();
			let value = doc.insert(&field_path, init_value)?;
			let out = func(value);
			self.commands.entity(doc_entity).insert(doc);
			Ok(out)
		} else {
			Err(DocumentError::ObjectKeyNotFound {
				path: field_path.clone(),
				key: format!("{:?}", field_path),
			}
			.into())
		}
	}

	/// Type-check a write of `T` against the document's [`DocumentSchema`].
	///
	/// Passes silently when the document has no schema.
	fn assert_field_type<T: Typed>(
		&mut self,
		subject: Entity,
		field: &FieldRef,
	) -> Result {
		let doc_entity = self.entity(subject, &field.document);
		// schema paths are authored against the resolved (absolute) path
		let field_path = self.scopes.resolved_path(subject, &field.field_path);
		if let Ok(schema) = self.schemas.get(doc_entity) {
			schema.assert_field_type::<T>(&field_path)?;
		}
		Ok(())
	}

	/// Type-check a list-item write of `T` against the document's
	/// [`DocumentSchema`]. Passes silently when the document has no schema.
	fn assert_list_item_type<T: Typed>(
		&mut self,
		subject: Entity,
		field: &FieldRef,
	) -> Result {
		let doc_entity = self.entity(subject, &field.document);
		// schema paths are authored against the resolved (absolute) path
		let field_path = self.scopes.resolved_path(subject, &field.field_path);
		if let Ok(schema) = self.schemas.get(doc_entity) {
			schema.assert_list_item_type::<T>(&field_path)?;
		}
		Ok(())
	}

	/// Set a field to a typed value, type-checked against the document schema.
	pub fn set_field_typed<T>(
		&mut self,
		subject: Entity,
		field: &FieldRef,
		value: &T,
	) -> Result
	where
		T: Serialize + Typed,
	{
		self.assert_field_type::<T>(subject, field)?;
		let new_value = Value::from_serde(value)?;
		self.with_field(subject, field, move |slot| *slot = new_value)
	}

	/// Append a typed value to a list field, type-checked against the document
	/// schema. Coerces a missing or null field into an empty list first.
	pub fn push_field<T>(
		&mut self,
		subject: Entity,
		field: &FieldRef,
		value: &T,
	) -> Result
	where
		T: Serialize + Typed,
	{
		self.assert_list_item_type::<T>(subject, field)?;
		let value = Value::from_serde(value)?;
		self.with_field(subject, field, move |slot| -> Result {
			as_list_mut(slot)?.push(value);
			Ok(())
		})?
	}

	/// Insert a typed value at an index of a list field, clamping out-of-range
	/// indices to the list length. Type-checked against the document schema and
	/// coerces a missing or null field into an empty list first.
	pub fn insert_at_field<T>(
		&mut self,
		subject: Entity,
		field: &FieldRef,
		index: usize,
		value: &T,
	) -> Result
	where
		T: Serialize + Typed,
	{
		self.assert_list_item_type::<T>(subject, field)?;
		let value = Value::from_serde(value)?;
		self.with_field(subject, field, move |slot| -> Result {
			let list = as_list_mut(slot)?;
			let index = index.min(list.len());
			list.insert(index, value);
			Ok(())
		})?
	}
}

/// Coerce a field [`Value`] into a mutable list, treating null as empty.
fn as_list_mut(value: &mut Value) -> Result<&mut Vec<Value>> {
	if value.is_null() {
		*value = Value::List(Vec::new());
	}
	match value {
		Value::List(list) => Ok(list),
		other => bevybail!("expected list, received {}", other.kind()),
	}
}



#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	#[beet_core::test]
	fn document_query_get_and_get_mut() {
		let mut world = World::new();
		let entity = world.spawn(Document::new(val!({ "value": 42i64 }))).id();

		// Test get
		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc =
						query.get(entity, &DocumentPath::Ancestor).unwrap();
					doc.get_field_ref(&[FieldSegment::key("value")])
						.unwrap()
						.as_i64()
						.unwrap()
						.xpect_eq(42);
				},
				entity,
			)
			.unwrap();

		// Test get_mut
		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let mut doc =
						query.get_mut(entity, &DocumentPath::Ancestor).unwrap();
					let val = doc
						.get_field_mut(&[FieldSegment::key("value")])
						.unwrap();
					*val = Value::Int(100);
				},
				entity,
			)
			.unwrap();

		// Verify mutation
		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc =
						query.get(entity, &DocumentPath::Ancestor).unwrap();
					doc.get_field::<i64>(&[FieldSegment::key("value")])
						.unwrap()
						.xpect_eq(100);
				},
				entity,
			)
			.unwrap();
	}

	#[beet_core::test]
	fn document_query_with_field() {
		let mut world = World::new();
		let entity = world.spawn(Document::new(val!({ "count": 5i64 }))).id();

		let field = FieldRef::new("count");

		world
			.run_system_cached_with(
				|In((entity, field)): In<(Entity, FieldRef)>,
				 mut query: DocumentQuery| {
					query
						.with_field(entity, &field, |value| {
							let current = value.as_i64().unwrap();
							*value = Value::Int(current + 1);
						})
						.unwrap();
				},
				(entity, field.clone()),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc =
						query.get(entity, &DocumentPath::Ancestor).unwrap();
					doc.get_field::<i64>(&[FieldSegment::key("count")])
						.unwrap()
						.xpect_eq(6);
				},
				entity,
			)
			.unwrap();
	}

	#[beet_core::test]
	fn document_query_with_field_initializes() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		let field = FieldRef::new("new_field");

		world
			.run_system_cached_with(
				|In((entity, field)): In<(Entity, FieldRef)>,
				 mut query: DocumentQuery| {
					query
						.with_field(entity, &field, |value| {
							*value = Value::Str("created".into());
						})
						.unwrap();
				},
				(entity, field.clone()),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc =
						query.get(entity, &DocumentPath::Ancestor).unwrap();
					doc.get_field::<String>(&[FieldSegment::key("new_field")])
						.unwrap()
						.xpect_eq("created");
				},
				entity,
			)
			.unwrap();
	}

	#[beet_core::test]
	fn document_query_resolve_card() {
		let mut world = World::new();
		let card = world
			.spawn(Document::new(val!({ "card_data": "test" })))
			.id();
		let child = world.spawn(ChildOf(card)).id();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc =
						query.get(entity, &DocumentPath::Ancestor).unwrap();
					doc.get_field::<String>(&[FieldSegment::key("card_data")])
						.unwrap()
						.xpect_eq("test");
				},
				child,
			)
			.unwrap();
	}

	#[beet_core::test]
	fn document_query_resolve_root() {
		let mut world = World::new();
		let root = world
			.spawn(Document::new(val!({ "root_data": "root_test" })))
			.id();
		let child = world.spawn(ChildOf(root)).id();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc = query.get(entity, &DocumentPath::Root).unwrap();
					doc.get_field::<String>(&[FieldSegment::key("root_data")])
						.unwrap()
						.xpect_eq("root_test");
				},
				child,
			)
			.unwrap();
	}
}
