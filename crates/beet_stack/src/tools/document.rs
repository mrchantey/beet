use crate::prelude::*;
use beet_core::prelude::*;


/// Document-related errors.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
	#[error("expected array, found {current:#?}\nAt path {path:?}")]
	ExpectedArray {
		current: serde_json::Value,
		path: Vec<FieldPath>,
	},
	#[error("array index {index} out of bounds\nAt path {path:?}")]
	ArrayIndexOutOfBounds { index: usize, path: Vec<FieldPath> },
	#[error("expected object, found {current:#?}\nat path {path:?}")]
	ExpectedObject {
		current: serde_json::Value,
		path: Vec<FieldPath>,
	},
	#[error("object key '{key}' not found\nAt path {path:?}")]
	ObjectKeyNotFound { key: String, path: Vec<FieldPath> },
	#[error("Failed to deserialize: '{error}'\nAt path {path:?}")]
	FailedToDeserialize { error: String, path: Vec<FieldPath> },
}

/// In-memory representation of a document.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Component)]
pub struct Document(serde_json::Value);

impl Document {
	/// Get a field from the document by path, deserializing to type T.
	pub fn get_field<T>(&self, path: &[FieldPath]) -> Result<T, DocumentError>
	where
		T: serde::de::DeserializeOwned,
	{
		let value = self.get_field_ref(path)?;
		serde_json::from_value(value.clone()).map_err(|err| {
			DocumentError::FailedToDeserialize {
				error: err.to_string(),
				path: path.to_vec(),
			}
		})
	}
	/// Get a field from the document by path, deserializing to type T.
	pub fn get_field_ref(
		&self,
		path: &[FieldPath],
	) -> Result<&serde_json::Value, DocumentError> {
		let mut current = &self.0;

		for segment in path {
			match segment {
				FieldPath::ArrayIndex(idx) => {
					current = current
						.as_array()
						.ok_or_else(|| DocumentError::ExpectedArray {
							current: current.clone(),
							path: path.to_vec(),
						})?
						.get(*idx)
						.ok_or_else(|| {
							DocumentError::ArrayIndexOutOfBounds {
								index: *idx,
								path: path.to_vec(),
							}
						})?;
				}
				FieldPath::ObjectKey(key) => {
					current = current
						.as_object()
						.ok_or_else(|| DocumentError::ExpectedObject {
							current: current.clone(),
							path: path.to_vec(),
						})?
						.get(key)
						.ok_or_else(|| DocumentError::ObjectKeyNotFound {
							key: key.clone(),
							path: path.to_vec(),
						})?;
				}
			}
		}
		current.xok()
	}

	/// Get a mutable field from the document by path.
	#[allow(unused)]
	pub fn get_field_mut(
		&mut self,
		path: &[FieldPath],
	) -> Result<&mut serde_json::Value, DocumentError> {
		let mut current = &mut self.0;

		for segment in path {
			match segment {
				FieldPath::ArrayIndex(idx) => {
					todo!("fix compile errors");
					current = current
						.as_array_mut()
						.ok_or_else(|| DocumentError::ExpectedArray {
							current: current.clone(),
							path: path.to_vec(),
						})?
						.get_mut(*idx)
						.ok_or_else(|| {
							DocumentError::ArrayIndexOutOfBounds {
								index: *idx,
								path: path.to_vec(),
							}
						})?;
				}
				FieldPath::ObjectKey(key) => {
					todo!("fix compile errors");
					current = current
						.as_object_mut()
						.ok_or_else(|| DocumentError::ExpectedObject {
							current: current.clone(),
							path: path.to_vec(),
						})?
						.get_mut(key)
						.ok_or_else(|| DocumentError::ObjectKeyNotFound {
							key: key.clone(),
							path: path.to_vec(),
						})?;
				}
			}
		}
		current.xok()
	}
}

/// Marker component for the global application-level document.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Component)]
pub struct AppDocument;


/// A reference to a specific field For tools that interact with documents,
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: Vec<FieldPath>,
	/// By default fields are initialized with a default value
	/// if missing, otherwise an error is triggered.
	pub error_on_missing: bool,
}




/// Specify the document to operate on, either the global application-level document,
/// or another by id.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub enum DocumentPath {
	/// The document for this card.
	#[default]
	Card,
	/// The global application-level document
	App,
	/// A specific document by entity id
	Entity(Entity),
}



/// A path to a specific field within a document
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum FieldPath {
	/// Access an array element by index
	ArrayIndex(usize),
	/// Access an object field by key
	ObjectKey(String),
}


/// System parameter for ergonomic interaction with documents.
#[derive(SystemParam)]
pub struct DocumentQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	doc_query: Query<'w, 's, &'static mut Document>,
	app_doc_query: Query<'w, 's, Entity, With<AppDocument>>,
	card_query: Query<'w, 's, &'static Card>,
	commands: Commands<'w, 's>,
}

impl<'w, 's> DocumentQuery<'w, 's> {
	/// Get the entity for the given [`DocumentPath`],
	/// which may or may not contain an initialized document.
	fn resolve_entity(
		&self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<Entity> {
		match path {
			DocumentPath::Card => self
				.ancestors
				.iter_ancestors(entity)
				.find(|entity| self.card_query.get(*entity).is_ok())
				.ok_or_else(|| {
					bevyhow!("no card ancestor found for entity {}", entity)
				})?,
			DocumentPath::App => self
				.app_doc_query
				.iter()
				.next()
				.ok_or_else(|| bevyhow!("no app document found"))?,
			DocumentPath::Entity(entity) => *entity,
		}
		.xok()
	}

	/// Get the entity for the given [`DocumentPath`],
	/// spawning the app document if needed.
	pub fn document_entity(
		&mut self,
		entity: Entity,
		path: &DocumentPath,
	) -> Entity {
		match path {
			DocumentPath::Card => self
				.ancestors
				.iter_ancestors(entity)
				.find(|entity| self.card_query.get(*entity).is_ok())
				.unwrap_or(entity),
			DocumentPath::App => self
				.app_doc_query
				.iter()
				.next()
				.unwrap_or_else(|| self.commands.spawn(AppDocument).id()),
			DocumentPath::Entity(entity) => *entity,
		}
	}

	/// Returns the query item for the document.
	pub fn get(
		&self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<&Document> {
		let doc_entity = self.resolve_entity(entity, path)?;
		self.doc_query.get(doc_entity)?.xok()
	}


	/// Returns the mutable query item for the document.
	pub fn get_mut(
		&mut self,
		entity: Entity,
		path: &DocumentPath,
	) -> Result<Mut<'_, Document>> {
		let doc_entity = self.resolve_entity(entity, path)?;
		self.doc_query.get_mut(doc_entity)?.xok()
	}

	/// Execute a function on a document asynchronously.
	pub async fn with_async<O>(
		&mut self,
		entity: AsyncEntity,
		document: &DocumentPath,
		func: impl 'static + Send + Sync + Fn(&mut Document) -> O,
	) -> Result<O>
	where
		O: 'static + Send + Sync,
	{
		let id = entity.id();
		let doc_path = document.clone();
		entity
			.world()
			.with_then(move |world| {
				world.run_system_cached_with(
					move |In((entity, doc_path)): In<(
						Entity,
						DocumentPath,
					)>,
					      mut doc_query: DocumentQuery| {
						let mut doc_mut =
							doc_query.get_mut(entity, &doc_path)?;
						func(&mut doc_mut).xok()
					},
					(id, doc_path),
				)
			})
			.await?
	}
}
