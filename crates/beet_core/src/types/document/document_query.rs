use crate::prelude::*;
use bevy::reflect::Typed;

/// Read-only resolver of a [`DocumentPath`] to the entity owning (or destined
/// to own) the document.
///
/// Split from [`DocumentQuery`] so syncs holding their own `Document` access
/// (eg [`sync_source_field_refs`](super::sync_source_field_refs)) can resolve
/// paths without conflicting borrows.
#[derive(SystemParam)]
pub struct DocumentResolver<'w, 's> {
	traverse: ElementTraverseQuery<'w, 's>,
	docs: Query<'w, 's, (), With<Document>>,
	props: Query<'w, 's, (), With<PropsDocument>>,
}

impl DocumentResolver<'_, '_> {
	/// Resolve a [`DocumentPath`] to the actual entity that owns the document.
	pub fn entity(&self, subject: Entity, path: &DocumentPath) -> Entity {
		match path {
			DocumentPath::Root => self.traverse.root(subject),
			// nearest ancestor document, skipping props stores so user document
			// scoping inside a template body is unaffected
			DocumentPath::Ancestor => self
				.traverse
				.ancestors_inclusive(subject)
				.find(|entity| {
					self.docs.contains(*entity) && !self.props.contains(*entity)
				})
				.unwrap_or_else(|| self.traverse.root(subject)),
			// nearest ancestor props store, ie a template's materialized props
			DocumentPath::Props => self
				.traverse
				.ancestors_inclusive(subject)
				.find(|entity| self.props.contains(*entity))
				.unwrap_or_else(|| self.traverse.root(subject)),
			DocumentPath::Entity(entity) => *entity,
			DocumentPath::This => subject,
		}
	}

	/// Resolve a [`DocumentPath`] starting *above* `subject` (its parent), for
	/// tag-site bindings whose subject entity carries the template's own props
	/// store. A parentless subject resolves from itself.
	pub fn entity_above(&self, subject: Entity, path: &DocumentPath) -> Entity {
		self.traverse
			.parent(subject)
			.unwrap_or(subject)
			.xmap(|start| self.entity(start, path))
	}
}

/// System parameter for working with documents.
///
/// Provides convenient methods for accessing and modifying documents
/// on entities, with automatic entity resolution based on [`DocumentPath`].
#[derive(SystemParam)]
pub struct DocumentQuery<'w, 's> {
	resolver: DocumentResolver<'w, 's>,
	doc_query: Query<'w, 's, &'static mut Document>,
	schemas: Query<'w, 's, &'static DocumentSchema>,
	/// Shared upward resolver for the [`DocumentScope`] prefix, so reads and
	/// writes scope through the same walk.
	scopes: ScopeQuery<'w, 's>,
	commands: Commands<'w, 's>,
}

impl<'w, 's> DocumentQuery<'w, 's> {
	/// Resolve a [`DocumentPath`] to the actual entity that owns the document.
	pub fn entity(&self, subject: Entity, path: &DocumentPath) -> Entity {
		self.resolver.entity(subject, path)
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

	/// Clone a field's [`Value`] out of its document, the read twin of
	/// [`Self::with_field`]: same document resolution, same scope prefix, same
	/// [`FieldRef::on_missing`] policy, but the document is never touched so a
	/// read cannot dirty it.
	///
	/// A document or field that does not exist answers with the
	/// [`OnMissingField::Init`] seed, ie `[]` for a list field, without writing
	/// it back.
	pub fn field_value(
		&mut self,
		subject: Entity,
		field: &FieldRef,
	) -> Result<Value> {
		let doc_entity = self.entity(subject, &field.document);
		let field_path = self.scopes.resolved_path(
			subject,
			&field.field_path,
			Some(doc_entity),
		);
		match self
			.doc_query
			.get(doc_entity)
			.ok()
			.and_then(|doc| doc.get_field_ref(&field_path).ok())
		{
			Some(value) => value.clone().xok(),
			None => match &field.on_missing {
				OnMissingField::Init { value } => value.clone().xok(),
				_ => Err(DocumentError::ObjectKeyNotFound {
					key: format!("{field_path}"),
					path: field_path,
				}
				.into()),
			},
		}
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
		let field_path = self.scopes.resolved_path(
			subject,
			&field.field_path,
			Some(doc_entity),
		);

		if let Ok(mut doc) = self.doc_query.get_mut(doc_entity) {
			let value = if let Ok(value) = doc.get_field_mut(&field_path) {
				value
			} else if let OnMissing::Default(init_value) = &field.on_missing {
				doc.insert(&field_path, init_value)?
			} else {
				return Err(DocumentError::ObjectKeyNotFound {
					path: field_path.clone(),
					key: format!("{:?}", field_path),
				}
				.into());
			};
			Ok(func(value))
		} else if let OnMissing::Default(init_value) = &field.on_missing {
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
		let field_path = self.scopes.resolved_path(
			subject,
			&field.field_path,
			Some(doc_entity),
		);
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
		let field_path = self.scopes.resolved_path(
			subject,
			&field.field_path,
			Some(doc_entity),
		);
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
			slot.as_list_mut_or_init()?.push(value);
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
			let list = slot.as_list_mut_or_init()?;
			let index = index.min(list.len());
			list.insert(index, value);
			Ok(())
		})?
	}
}


#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	#[crate::test]
	fn document_query_get_and_get_mut() {
		let mut world = World::new();
		let entity =
			world.spawn(Document::new(value!({ "value": 42i64 }))).id();

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

	#[crate::test]
	fn document_query_with_field() {
		let mut world = World::new();
		let entity = world.spawn(Document::new(value!({ "count": 5i64 }))).id();

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

	#[crate::test]
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

	#[crate::test]
	fn document_query_resolve_card() {
		let mut world = World::new();
		let card = world
			.spawn(Document::new(value!({ "card_data": "test" })))
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

	#[crate::test]
	fn document_query_resolve_root() {
		let mut world = World::new();
		let root = world
			.spawn(Document::new(value!({ "root_data": "root_test" })))
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
