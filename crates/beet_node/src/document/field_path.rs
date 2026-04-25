use beet_core::prelude::*;

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldPath(Vec<FieldSegment>);

impl FieldPath {
	pub fn new<T>(segments: impl IntoIterator<Item = T>) -> Self
	where
		T: Into<FieldSegment>,
	{
		Self(segments.into_iter().map(Into::into).collect())
	}

	/// Splits by double colons `::`
	pub fn from_module_path(val: &'static str) -> Self {
		val.split("::")
			.map(|s| FieldSegment::ObjectKey(SmolStr::new_static(s)))
			.collect::<Vec<_>>()
			.into()
	}

	pub fn of<T: TypePath>() -> Self { Self::from_module_path(T::type_path()) }
}

impl From<Vec<FieldSegment>> for FieldPath {
	fn from(segments: Vec<FieldSegment>) -> Self { Self(segments) }
}
impl From<&[FieldSegment]> for FieldPath {
	fn from(segments: &[FieldSegment]) -> Self { Self(segments.to_vec()) }
}

impl std::fmt::Display for FieldPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let segments = self
			.0
			.iter()
			.map(|seg| match seg {
				FieldSegment::ArrayIndex(i) => format!("[{}]", i),
				FieldSegment::ObjectKey(k) => k.to_string(),
			})
			.collect::<Vec<_>>()
			.join(".");
		write!(f, "{}", segments)
	}
}

/// A path segment for navigating [`Value`] structures.
///
/// Paths are built from sequences of these segments to access nested fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl std::fmt::Display for FieldSegment {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FieldSegment::ArrayIndex(i) => write!(f, "{}", i),
			FieldSegment::ObjectKey(k) => write!(f, "{}", k),
		}
	}
}

impl From<&str> for FieldSegment {
	fn from(s: &str) -> Self { Self::key(s) }
}
impl From<&&str> for FieldSegment {
	fn from(s: &&str) -> Self { Self::key(*s) }
}
impl From<String> for FieldSegment {
	fn from(s: String) -> Self { Self::key(s) }
}
impl From<usize> for FieldSegment {
	fn from(i: usize) -> Self { Self::index(i) }
}
impl From<u32> for FieldSegment {
	fn from(i: u32) -> Self { Self::index(i as usize) }
}
impl From<u64> for FieldSegment {
	fn from(i: u64) -> Self { Self::index(i as usize) }
}
impl From<i32> for FieldSegment {
	fn from(i: i32) -> Self { Self::index(i as usize) }
}
impl From<i64> for FieldSegment {
	fn from(i: i64) -> Self { Self::index(i as usize) }
}

/// Convert various types into a field path vector for document navigation.
pub trait IntoFieldPath<M> {
	/// Convert this type into a vector of field path segments.
	fn into_field_path(self) -> FieldPath;
}
impl IntoFieldPath<Self> for FieldPath {
	fn into_field_path(self) -> FieldPath { self }
}
pub struct IteratorIntoFieldPathMarker;

impl<T, U> IntoFieldPath<IteratorIntoFieldPathMarker> for T
where
	T: IntoIterator<Item = U>,
	U: Into<FieldSegment>,
{
	fn into_field_path(self) -> FieldPath {
		self.into_iter().map(Into::into).collect::<Vec<_>>().into()
	}
}

impl IntoFieldPath<Self> for &[FieldSegment] {
	fn into_field_path(self) -> FieldPath { self.to_vec().into() }
}
impl IntoFieldPath<Self> for &str {
	fn into_field_path(self) -> FieldPath {
		vec![FieldSegment::key(self)].into()
	}
}
impl IntoFieldPath<Self> for String {
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
