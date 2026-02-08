use crate::prelude::*;
use beet_core::prelude::*;

/// A reference to a specific field in a document.
///
/// Used by content and tools to interact with document fields. By default,
/// fields are initialized with `null` if missing, unless configured otherwise
/// via [`on_missing`](FieldRef::on_missing).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[component(immutable)]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: Vec<FieldPath>,
	/// Behavior when the field is missing from the document.
	pub on_missing: OnMissingField,
}


impl FieldRef {
	/// Create a new field reference with the given field path.
	///
	/// Uses the default [`DocumentPath::Card`] for document resolution.
	/// Use [`with_document`](Self::with_document) to specify a different document.
	///
	/// By default, missing fields are initialized with [`Value::Null`].
	pub fn new(field_path: impl IntoFieldPathVec) -> Self {
		Self {
			document: DocumentPath::default(),
			field_path: field_path.into_field_path_vec(),
			on_missing: OnMissingField::default(),
		}
	}

	/// Set the document path for this field reference.
	pub fn with_document(mut self, document: DocumentPath) -> Self {
		self.document = document;
		self
	}

	/// Set this field reference to error if the field is missing instead of initializing it.
	pub fn error_on_missing(mut self) -> Self {
		self.on_missing = OnMissingField::EmitError;
		self
	}

	/// Create a tuple of this field reference and an empty text content.
	pub fn as_text(&self) -> (Self, TextContent) {
		(self.clone(), TextContent::default())
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

#[cfg(test)]
mod test {
	use super::*;



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
		let field = FieldRef::new("field");

		field.document.xpect_eq(DocumentPath::Card);
		field
			.field_path
			.xpect_eq(vec![FieldPath::ObjectKey("field".to_string())]);
		field
			.on_missing
			.xpect_eq(OnMissingField::Init { value: Value::Null });
	}
}
