use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::FromReflect;
use bevy::reflect::Typed;

/// In-memory document that can be attached to entities.
///
/// Documents provide structured storage for cards and other entities,
/// similar to document databases. Fields can be accessed and modified
/// using [`FieldPath`] to navigate nested structures.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Deref,
	DerefMut,
	Component,
	Reflect,
)]
#[reflect(Component)]
pub struct Document(pub Value);

impl Document {
	/// Create a new document from a [`Value`].
	pub fn new(value: Value) -> Self { Self(value) }

	/// Create a document from a reflectable value.
	///
	/// ## Errors
	///
	/// Returns an error if reflection conversion fails.
	pub fn from_reflect<T: bevy::reflect::PartialReflect>(
		value: &T,
	) -> Result<Self> {
		Value::from_reflect(value).map(Self)
	}

	/// Initialize a nested field in a document unless there is a type clash.
	/// Arrays and objects are initialized with fields and items, as required by the path.
	/// An array or object will only be initialized if the current value is [`None`] or [`Null`].
	///
	/// ## Errors
	///
	/// Errors if an array or object is expected, and the actual type is not the expected, nor empty.
	fn try_init_field_with(
		&mut self,
		path: &[FieldPath],
		init_value: &Value,
	) -> Result<&mut Value> {
		let mut current = &mut self.0;

		for segment in path {
			match segment {
				FieldPath::ArrayIndex(idx) => {
					// initialize as array if null or empty
					if current.is_null() {
						*current = Value::List(Vec::new());
					}
					let current_clone = current.clone();
					let path_clone = path.to_vec();
					let array = current.as_list_mut().ok_or_else(|| {
						DocumentError::ExpectedArray {
							current: current_clone,
							path: path_clone,
						}
					})?;
					// expand array if needed
					while array.len() <= *idx {
						array.push(Value::Null);
					}
					current = &mut array[*idx];
				}
				FieldPath::ObjectKey(key) => {
					// initialize as object if null or empty
					if current.is_null() {
						*current = Value::Map(Default::default());
					}
					let current_clone = current.clone();
					let path_clone = path.to_vec();
					let object = current.as_map_mut().ok_or_else(|| {
						DocumentError::ExpectedObject {
							current: current_clone,
							path: path_clone,
						}
					})?;
					if !object.contains_key(key) {
						object.insert(key.clone(), init_value.clone());
					}
					current = object.get_mut(key).unwrap();
				}
			}
		}
		current.xok()
	}

	/// Get a field from the document by path, converting to type `T`.
	///
	/// ## Errors
	///
	/// Returns an error if the path doesn't exist, the type is incorrect,
	/// or conversion fails.
	pub fn get_field<T>(&self, path: &[FieldPath]) -> Result<T, DocumentError>
	where
		T: 'static + Send + Sync + FromReflect + Typed,
	{
		let value = self.get_field_ref(path)?;
		value
			.into_reflect()
			.map_err(|err| DocumentError::FailedToDeserialize {
				error: err.to_string(),
				path: path.to_vec(),
			})
	}
	/// Get a reference to a field in the document by path.
	///
	/// ## Errors
	///
	/// Returns an error if the path doesn't exist or encounters a type mismatch.
	pub fn get_field_ref(
		&self,
		path: &[FieldPath],
	) -> Result<&Value, DocumentError> {
		let mut current = &self.0;

		for segment in path {
			match segment {
				FieldPath::ArrayIndex(idx) => {
					current = current
						.as_list()
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
						.as_map()
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

	/// Get a mutable reference to a field in the document by path.
	///
	/// ## Errors
	///
	/// Returns an error if the path doesn't exist or encounters a type mismatch.
	pub fn get_field_mut(
		&mut self,
		path: &[FieldPath],
	) -> Result<&mut Value, DocumentError> {
		let mut current = &mut self.0;

		for segment in path {
			match segment {
				FieldPath::ArrayIndex(idx) => {
					let idx_val = *idx;
					// Check type first before attempting mutation
					if !current.is_list() {
						return Err(DocumentError::ExpectedArray {
							current: current.clone(),
							path: path.to_vec(),
						});
					}
					let array = current.as_list_mut().unwrap();
					if idx_val >= array.len() {
						return Err(DocumentError::ArrayIndexOutOfBounds {
							index: idx_val,
							path: path.to_vec(),
						});
					}
					current = &mut array[idx_val];
				}
				FieldPath::ObjectKey(key) => {
					// Check type first before attempting mutation
					if !current.is_map() {
						return Err(DocumentError::ExpectedObject {
							current: current.clone(),
							path: path.to_vec(),
						});
					}
					let object = current.as_map_mut().unwrap();
					if !object.contains_key(key) {
						return Err(DocumentError::ObjectKeyNotFound {
							key: key.clone(),
							path: path.to_vec(),
						});
					}
					current = object.get_mut(key).unwrap();
				}
			}
		}
		current.xok()
	}
}


/// Errors that can occur when working with documents.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
	/// Expected an array but found a different type at the given path.
	#[error("expected array, found {current:?}\nAt path {path:?}")]
	ExpectedArray {
		/// The actual value that was found.
		current: Value,
		/// The path where the error occurred.
		path: Vec<FieldPath>,
	},
	/// Array index was out of bounds.
	#[error("array index {index} out of bounds\nAt path {path:?}")]
	ArrayIndexOutOfBounds {
		/// The index that was out of bounds.
		index: usize,
		/// The path where the error occurred.
		path: Vec<FieldPath>,
	},
	/// Expected an object but found a different type at the given path.
	#[error("expected object, found {current:?}\nat path {path:?}")]
	ExpectedObject {
		/// The actual value that was found.
		current: Value,
		/// The path where the error occurred.
		path: Vec<FieldPath>,
	},
	/// Object key was not found at the given path.
	#[error("object key '{key}' not found\nAt path {path:?}")]
	ObjectKeyNotFound {
		/// The key that was not found.
		key: String,
		/// The path where the error occurred.
		path: Vec<FieldPath>,
	},
	/// Failed to deserialize value to the requested type.
	#[error("Failed to deserialize: '{error}'\nAt path {path:?}")]
	FailedToDeserialize {
		/// The deserialization error message.
		error: String,
		/// The path where the error occurred.
		path: Vec<FieldPath>,
	},
}


/// Specifies behavior when a field is missing from a document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum OnMissingField {
	/// Initialize the field with the provided value if it doesn't exist.
	Init {
		/// The value to initialize the field with.
		value: Value,
	},
	/// Emit an error if the field doesn't exist.
	EmitError,
}

impl Default for OnMissingField {
	fn default() -> Self { Self::Init { value: Value::Null } }
}

/// A reference to a specific field in a document.
///
/// Used by content and tools to interact with document fields. By default,
/// fields are initialized with `null` if missing, unless configured otherwise
/// via [`on_missing`](FieldRef::on_missing).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: Vec<FieldPath>,
	/// Behavior when the field is missing from the document.
	pub on_missing: OnMissingField,
}


impl FieldRef {
	/// Create a new field reference with the given document path and field path.
	///
	/// By default, missing fields are initialized with `null`.
	pub fn new(
		document: DocumentPath,
		field_path: impl IntoFieldPathVec,
	) -> Self {
		Self {
			document,
			field_path: field_path.into_field_path_vec(),
			on_missing: OnMissingField::default(),
		}
	}

	/// Set this field reference to error if the field is missing instead of initializing it.
	pub fn error_on_missing(mut self) -> Self {
		self.on_missing = OnMissingField::EmitError;
		self
	}

	/// Set the behavior when the field is missing.
	pub fn on_missing(mut self, on_missing: OnMissingField) -> Self {
		self.on_missing = on_missing;
		self
	}

	/// Set the field to initialize with a specific value if missing.
	pub fn init_with(mut self, value: Value) -> Self {
		self.on_missing = OnMissingField::Init { value };
		self
	}
}

/// Specifies which document to operate on.
///
/// Documents can be attached to cards, the root entity, or any specific entity.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub enum DocumentPath {
	/// The document for this card.
	#[default]
	Card,
	/// The root entity.
	Root,
	/// A specific document by entity id
	Entity(Entity),
}



/// A path segment for navigating document structure.
///
/// Paths are built from sequences of these segments to access nested fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum FieldPath {
	/// Access an array element by index.
	ArrayIndex(usize),
	/// Access an object field by key.
	ObjectKey(String),
}


/// Convert various types into a field path vector for document navigation.
pub trait IntoFieldPathVec {
	/// Convert this type into a vector of field path segments.
	fn into_field_path_vec(self) -> Vec<FieldPath>;
}
impl IntoFieldPathVec for Vec<FieldPath> {
	fn into_field_path_vec(self) -> Vec<FieldPath> { self }
}
impl IntoFieldPathVec for Vec<String> {
	fn into_field_path_vec(self) -> Vec<FieldPath> {
		self.into_iter().map(FieldPath::ObjectKey).collect()
	}
}
impl IntoFieldPathVec for Vec<&str> {
	fn into_field_path_vec(self) -> Vec<FieldPath> {
		self.into_iter()
			.map(|s| FieldPath::ObjectKey(s.to_string()))
			.collect()
	}
}

impl IntoFieldPathVec for Vec<usize> {
	fn into_field_path_vec(self) -> Vec<FieldPath> {
		self.into_iter().map(FieldPath::ArrayIndex).collect()
	}
}

impl IntoFieldPathVec for &[FieldPath] {
	fn into_field_path_vec(self) -> Vec<FieldPath> { self.to_vec() }
}
impl IntoFieldPathVec for &str {
	fn into_field_path_vec(self) -> Vec<FieldPath> {
		vec![FieldPath::ObjectKey(self.to_string())]
	}
}
impl IntoFieldPathVec for String {
	fn into_field_path_vec(self) -> Vec<FieldPath> {
		vec![FieldPath::ObjectKey(self)]
	}
}

/// System parameter for working with documents.
///
/// Provides convenient methods for accessing and modifying documents
/// on entities, with automatic entity resolution based on [`DocumentPath`].
#[derive(SystemParam)]
pub struct DocumentQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	doc_query: Query<'w, 's, &'static mut Document>,
	card_query: Query<'w, 's, &'static Card>,
	commands: Commands<'w, 's>,
}

impl<'w, 's> DocumentQuery<'w, 's> {
	/// Resolve a [`DocumentPath`] to the actual entity that owns the document.
	fn resolve_entity(
		&mut self,
		subject: Entity,
		path: &DocumentPath,
	) -> Entity {
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
		let doc_entity = self.resolve_entity(entity, path);
		self.doc_query.get(doc_entity)?.xok()
	}


	/// Returns the mutable query item for the document.
	pub fn get_mut(
		&mut self,
		subject: Entity,
		path: &DocumentPath,
	) -> Result<Mut<'_, Document>> {
		let doc_entity = self.resolve_entity(subject, path);
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
		let doc_entity = self.resolve_entity(subject, &field.document);

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
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn document_get_field_ref() {
		let doc = Document::new(val!({
			"name": "Test",
			"count": 42i64,
			"nested": { "value": "deep" }
		}));

		doc.get_field_ref(&[FieldPath::ObjectKey("name".to_string())])
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("Test");

		doc.get_field_ref(&[FieldPath::ObjectKey("count".to_string())])
			.unwrap()
			.as_i64()
			.unwrap()
			.xpect_eq(42);

		doc.get_field_ref(&[
			FieldPath::ObjectKey("nested".to_string()),
			FieldPath::ObjectKey("value".to_string()),
		])
		.unwrap()
		.as_str()
		.unwrap()
		.xpect_eq("deep");
	}

	#[test]
	fn document_get_field() {
		let doc = Document::new(val!({
			"name": "Test",
			"count": 42i64
		}));

		doc.get_field::<String>(&[FieldPath::ObjectKey("name".to_string())])
			.unwrap()
			.xpect_eq("Test");

		doc.get_field::<i64>(&[FieldPath::ObjectKey("count".to_string())])
			.unwrap()
			.xpect_eq(42);
	}

	#[test]
	fn document_get_field_array() {
		let doc = Document::new(val!({
			"items": [1i64, 2i64, 3i64, 4i64, 5i64]
		}));

		doc.get_field_ref(&[
			FieldPath::ObjectKey("items".to_string()),
			FieldPath::ArrayIndex(2),
		])
		.unwrap()
		.as_i64()
		.unwrap()
		.xpect_eq(3);
	}

	#[test]
	fn document_get_field_mut() {
		let mut doc = Document::new(val!({ "count": 10i64 }));

		let value = doc
			.get_field_mut(&[FieldPath::ObjectKey("count".to_string())])
			.unwrap();
		*value = Value::I64(20);

		doc.get_field::<i64>(&[FieldPath::ObjectKey("count".to_string())])
			.unwrap()
			.xpect_eq(20);
	}

	#[test]
	fn document_try_init_field_object() {
		let mut doc = Document::default();

		let value = doc
			.try_init_field_with(
				&[
					FieldPath::ObjectKey("nested".to_string()),
					FieldPath::ObjectKey("value".to_string()),
				],
				&Value::Null,
			)
			.unwrap();

		*value = Value::String("initialized".to_string());

		doc.get_field::<String>(&[
			FieldPath::ObjectKey("nested".to_string()),
			FieldPath::ObjectKey("value".to_string()),
		])
		.unwrap()
		.xpect_eq("initialized");
	}

	#[test]
	fn document_try_init_field_array() {
		let mut doc = Document::default();

		let value = doc
			.try_init_field_with(
				&[
					FieldPath::ObjectKey("items".to_string()),
					FieldPath::ArrayIndex(2),
				],
				&Value::Null,
			)
			.unwrap();

		*value = Value::I64(42);

		doc.get_field::<i64>(&[
			FieldPath::ObjectKey("items".to_string()),
			FieldPath::ArrayIndex(2),
		])
		.unwrap()
		.xpect_eq(42);
	}

	#[test]
	fn field_path_conversion() {
		let string_vec: Vec<FieldPath> =
			vec!["a".to_string(), "b".to_string()].into_field_path_vec();
		string_vec.xpect_eq(vec![
			FieldPath::ObjectKey("a".to_string()),
			FieldPath::ObjectKey("b".to_string()),
		]);

		let str_vec: Vec<FieldPath> = vec!["x", "y"].into_field_path_vec();
		str_vec.xpect_eq(vec![
			FieldPath::ObjectKey("x".to_string()),
			FieldPath::ObjectKey("y".to_string()),
		]);

		let index_vec: Vec<FieldPath> = vec![0, 1, 2].into_field_path_vec();
		index_vec.xpect_eq(vec![
			FieldPath::ArrayIndex(0),
			FieldPath::ArrayIndex(1),
			FieldPath::ArrayIndex(2),
		]);
	}

	#[test]
	fn field_ref_new() {
		let field = FieldRef::new(DocumentPath::Card, "field");

		field.document.xpect_eq(DocumentPath::Card);
		field
			.field_path
			.xpect_eq(vec![FieldPath::ObjectKey("field".to_string())]);
		field
			.on_missing
			.xpect_eq(OnMissingField::Init { value: Value::Null });
	}

	#[test]
	fn field_ref_error_on_missing() {
		let field =
			FieldRef::new(DocumentPath::Card, "field").error_on_missing();
		field.on_missing.xpect_eq(OnMissingField::EmitError);
	}

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
			.spawn((Card, Document::new(val!({ "count": 5i64 }))))
			.id();

		let field = FieldRef::new(DocumentPath::Card, "count");

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
		let entity = world.spawn(Card).id();

		let field = FieldRef::new(DocumentPath::Card, "new_field");

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
			.spawn((Card, Document::new(val!({ "card_data": "test" }))))
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
