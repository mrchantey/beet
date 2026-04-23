use crate::prelude::*;

/// A path to a specific field within a [`Value`].
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


/// A path segment for navigating [`Value`] structures.
///
/// Paths are built from sequences of these segments to access nested fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum FieldSegment {
	/// Access an array element by index.
	ArrayIndex(usize),
	/// Access an object field by key.
	ObjectKey(SmolStr),
}
impl FieldSegment {
	/// Create a field segment for an object key.
	pub fn key(key: impl Into<SmolStr>) -> Self { Self::ObjectKey(key.into()) }
	/// Create a field segment for an array index.
	pub fn index(index: usize) -> Self { Self::ArrayIndex(index) }
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
			.map(FieldSegment::key)
			.collect::<Vec<_>>()
			.into()
	}
}
impl IntoFieldPath for Vec<&str> {
	fn into_field_path(self) -> FieldPath {
		self.into_iter()
			.map(FieldSegment::key)
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
		vec![FieldSegment::key(self)].into()
	}
}
impl IntoFieldPath for String {
	fn into_field_path(self) -> FieldPath {
		vec![FieldSegment::key(self)].into()
	}
}

#[cfg(test)]
mod test {
	use super::*;



	#[test]
	fn field_path_conversion() {
		let string_vec =
			vec!["a".to_string(), "b".to_string()].into_field_path();
		string_vec
			.0
			.xpect_eq(vec![FieldSegment::key("a"), FieldSegment::key("b")]);

		let str_vec = vec!["x", "y"].into_field_path();
		str_vec
			.0
			.xpect_eq(vec![FieldSegment::key("x"), FieldSegment::key("y")]);

		let index_vec = vec![0, 1, 2].into_field_path();
		index_vec.0.xpect_eq(vec![
			FieldSegment::index(0),
			FieldSegment::index(1),
			FieldSegment::index(2),
		]);
	}
}
