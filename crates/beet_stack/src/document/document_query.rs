use crate::prelude::*;
use beet_core::prelude::*;

/// System parameter for working with documents.
///
/// Provides convenient methods for accessing and modifying documents
/// on entities, with automatic entity resolution based on [`DocumentPath`].
#[derive(SystemParam)]
pub struct DocumentQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	doc_query: Query<'w, 's, &'static mut Document>,
	card_query: Query<'w, 's, &'static CardTool>,
	commands: Commands<'w, 's>,
}

impl<'w, 's> DocumentQuery<'w, 's> {
	/// Resolve a [`DocumentPath`] to the actual entity that owns the document.
	pub fn entity(&mut self, subject: Entity, path: &DocumentPath) -> Entity {
		match path {
			DocumentPath::Root => self.ancestors.root_ancestor(subject),
			DocumentPath::Card => self
				.ancestors
				.iter_ancestors(subject)
				.find(|entity| self.card_query.get(*entity).is_ok())
				.unwrap_or_else(|| self.ancestors.root_ancestor(subject)),
			DocumentPath::Entity(entity) => *entity,
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

	/// Execute a function on a document asynchronously.
	pub async fn with_async<O>(
		&mut self,
		entity: AsyncEntity,
		path: &DocumentPath,
		func: impl 'static + Send + Sync + Fn(&mut Document) -> O,
	) -> Result<O>
	where
		O: 'static + Send + Sync,
	{
		let id = entity.id();
		let path = path.clone();
		entity
			.world()
			.with_then(move |world| {
				world.run_system_cached_with(
					move |In((entity, path)): In<(Entity, DocumentPath)>,
					      mut query: DocumentQuery| {
						let mut doc = query.get_mut(entity, &path)?;
						func(&mut doc).xok()
					},
					(id, path),
				)
			})
			.await?
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

		if let Ok(mut doc) = self.doc_query.get_mut(doc_entity) {
			let value = if let Ok(value) = doc.get_field_mut(&field.field_path)
			{
				value
			} else if let OnMissingField::Init { value: init_value } =
				&field.on_missing
			{
				doc.try_init_field_with(&field.field_path, init_value)?
			} else {
				return Err(DocumentError::ObjectKeyNotFound {
					path: field.field_path.clone(),
					key: format!("{:?}", field.field_path),
				}
				.into());
			};
			Ok(func(value))
		} else if let OnMissingField::Init { value: init_value } =
			&field.on_missing
		{
			// create the document and run the method with it
			let mut doc = Document::default();
			let value =
				doc.try_init_field_with(&field.field_path, init_value)?;
			let out = func(value);
			self.commands.entity(doc_entity).insert(doc);
			Ok(out)
		} else {
			Err(DocumentError::ObjectKeyNotFound {
				path: field.field_path.clone(),
				key: format!("{:?}", field.field_path),
			}
			.into())
		}
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn document_query_get_and_get_mut() {
		let mut world = World::new();
		let entity = world.spawn(Document::new(val!({ "value": 42i64 }))).id();

		// Test get
		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc = query.get(entity, &DocumentPath::Card).unwrap();
					doc.get_field_ref(&[FieldPath::ObjectKey(
						"value".to_string(),
					)])
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
						query.get_mut(entity, &DocumentPath::Card).unwrap();
					let val = doc
						.get_field_mut(&[FieldPath::ObjectKey(
							"value".to_string(),
						)])
						.unwrap();
					*val = Value::I64(100);
				},
				entity,
			)
			.unwrap();

		// Verify mutation
		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc = query.get(entity, &DocumentPath::Card).unwrap();
					doc.get_field::<i64>(&[FieldPath::ObjectKey(
						"value".to_string(),
					)])
					.unwrap()
					.xpect_eq(100);
				},
				entity,
			)
			.unwrap();
	}

	#[test]
	fn document_query_with_field() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, Document::new(val!({ "count": 5i64 }))))
			.id();

		let field = FieldRef::new("count");

		world
			.run_system_cached_with(
				|In((entity, field)): In<(Entity, FieldRef)>,
				 mut query: DocumentQuery| {
					query
						.with_field(entity, &field, |value| {
							let current = value.as_i64().unwrap();
							*value = Value::I64(current + 1);
						})
						.unwrap();
				},
				(entity, field.clone()),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc = query.get(entity, &DocumentPath::Card).unwrap();
					doc.get_field::<i64>(&[FieldPath::ObjectKey(
						"count".to_string(),
					)])
					.unwrap()
					.xpect_eq(6);
				},
				entity,
			)
			.unwrap();
	}

	#[test]
	fn document_query_with_field_initializes() {
		let mut world = World::new();
		let entity = world.spawn(CardTool).id();

		let field = FieldRef::new("new_field");

		world
			.run_system_cached_with(
				|In((entity, field)): In<(Entity, FieldRef)>,
				 mut query: DocumentQuery| {
					query
						.with_field(entity, &field, |value| {
							*value = Value::String("created".to_string());
						})
						.unwrap();
				},
				(entity, field.clone()),
			)
			.unwrap();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc = query.get(entity, &DocumentPath::Card).unwrap();
					doc.get_field::<String>(&[FieldPath::ObjectKey(
						"new_field".to_string(),
					)])
					.unwrap()
					.xpect_eq("created");
				},
				entity,
			)
			.unwrap();
	}

	#[test]
	fn document_query_resolve_card() {
		let mut world = World::new();
		let card = world
			.spawn((CardTool, Document::new(val!({ "card_data": "test" }))))
			.id();
		let child = world.spawn(ChildOf(card)).id();

		world
			.run_system_cached_with(
				|In(entity): In<Entity>, mut query: DocumentQuery| {
					let doc = query.get(entity, &DocumentPath::Card).unwrap();
					doc.get_field::<String>(&[FieldPath::ObjectKey(
						"card_data".to_string(),
					)])
					.unwrap()
					.xpect_eq("test");
				},
				child,
			)
			.unwrap();
	}

	#[test]
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
					doc.get_field::<String>(&[FieldPath::ObjectKey(
						"root_data".to_string(),
					)])
					.unwrap()
					.xpect_eq("root_test");
				},
				child,
			)
			.unwrap();
	}
}
