use crate::prelude::*;
use beet_core::prelude::*;

/// A reference to a specific field in a document.
///
/// Used by content and actions to interact with document fields. By default,
/// fields are initialized with `null` if missing, unless configured otherwise
/// via [`on_missing`](FieldRef::on_missing).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[component(immutable, on_add=on_add)]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: FieldPath,
	/// Behavior when the field is missing from the document.
	pub on_missing: OnMissingField,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	if !world.entity(cx.entity).contains::<Value>() {
		let this = world.entity(cx.entity).get::<FieldRef>().unwrap();
		let value = match this.on_missing.clone() {
			OnMissingField::Init { value } => value,
			_ => Value::default(),
		};
		world.commands().entity(cx.entity).insert(value);
	}
}


impl FieldRef {
	/// Create a new field reference with the given field path.
	///
	/// Uses the default [`DocumentPath::Ancestor`] for document resolution.
	/// Use [`with_document`](Self::with_document) to specify a different document.
	///
	/// By default, missing fields are initialized with [`Value::Null`].
	pub fn new(field_path: impl IntoFieldPath) -> Self {
		Self {
			document: DocumentPath::default(),
			field_path: field_path.into_field_path(),
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

	/// Set the behavior when the field is missing.
	pub fn on_missing(mut self, on_missing: OnMissingField) -> Self {
		self.on_missing = on_missing;
		self
	}

	/// Set the field to initialize with a specific value if missing.
	pub fn init_with(mut self, value: impl Into<Value>) -> Self {
		self.on_missing = OnMissingField::Init {
			value: value.into(),
		};
		self
	}
}

#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Reflect,
)]
pub struct FieldPath(Vec<FieldSegment>);

impl From<Vec<FieldSegment>> for FieldPath {
	fn from(segments: Vec<FieldSegment>) -> Self { Self(segments) }
}


/// A path segment for navigating document structure.
///
/// Paths are built from sequences of these segments to access nested fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum FieldSegment {
	/// Access an array element by index.
	ArrayIndex(usize),
	/// Access an object field by key.
	ObjectKey(String),
}


/// Convert various types into a field path vector for document navigation.
pub trait IntoFieldPath {
	/// Convert this type into a vector of field path segments.
	fn into_field_path(self) -> FieldPath;
}
impl IntoFieldPath for Vec<FieldSegment> {
	fn into_field_path(self) -> FieldPath { self.into() }
}
impl IntoFieldPath for Vec<String> {
	fn into_field_path(self) -> FieldPath {
		self.into_iter()
			.map(FieldSegment::ObjectKey)
			.collect::<Vec<_>>()
			.into()
	}
}
impl IntoFieldPath for Vec<&str> {
	fn into_field_path(self) -> FieldPath {
		self.into_iter()
			.map(|s| FieldSegment::ObjectKey(s.to_string()))
			.collect::<Vec<_>>()
			.into()
	}
}

impl IntoFieldPath for Vec<usize> {
	fn into_field_path(self) -> FieldPath {
		self.into_iter()
			.map(FieldSegment::ArrayIndex)
			.collect::<Vec<_>>()
			.into()
	}
}

impl IntoFieldPath for &[FieldSegment] {
	fn into_field_path(self) -> FieldPath { self.to_vec().into() }
}
impl IntoFieldPath for &str {
	fn into_field_path(self) -> FieldPath {
		vec![FieldSegment::ObjectKey(self.to_string())].into()
	}
}
impl IntoFieldPath for String {
	fn into_field_path(self) -> FieldPath {
		vec![FieldSegment::ObjectKey(self)].into()
	}
}

#[cfg(test)]
mod test {
	use super::*;



	#[test]
	fn field_path_conversion() {
		let string_vec =
			vec!["a".to_string(), "b".to_string()].into_field_path();
		string_vec.0.xpect_eq(vec![
			FieldSegment::ObjectKey("a".to_string()),
			FieldSegment::ObjectKey("b".to_string()),
		]);

		let str_vec = vec!["x", "y"].into_field_path();
		str_vec.0.xpect_eq(vec![
			FieldSegment::ObjectKey("x".to_string()),
			FieldSegment::ObjectKey("y".to_string()),
		]);

		let index_vec = vec![0, 1, 2].into_field_path();
		index_vec.0.xpect_eq(vec![
			FieldSegment::ArrayIndex(0),
			FieldSegment::ArrayIndex(1),
			FieldSegment::ArrayIndex(2),
		]);
	}

	#[test]
	fn field_ref_new() {
		let field = FieldRef::new("field");

		field.document.xpect_eq(DocumentPath::Ancestor);
		field
			.field_path
			.0
			.xpect_eq(vec![FieldSegment::ObjectKey("field".to_string())]);
		field
			.on_missing
			.xpect_eq(OnMissingField::Init { value: Value::Null });
	}
}
