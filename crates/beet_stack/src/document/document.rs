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
	pub(super) fn try_init_field_with(
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
}
