use beet_core::prelude::*;


/// used either as an element node, aka an xml text node, or as an attribute value.
/// a [`Value`] added to the same entity as an [`Element`] should be rendered as the first child.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum Value {
	#[default]
	Null,
	Bool(bool),
	Int(i64),
	Uint(u64),
	Float(Float),
	Bytes(Vec<u8>),
	Str(String),
}

impl Value {
	pub fn new(value: impl Into<Self>) -> Self { value.into() }
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Null => write!(f, "null"),
			Value::Bool(b) => write!(f, "{}", b),
			Value::Int(i) => write!(f, "{}", i),
			Value::Uint(u) => write!(f, "{}", u),
			Value::Float(fl) => write!(f, "{}", fl.0),
			Value::Bytes(bytes) => write!(f, "{:?}", bytes),
			Value::Str(s) => write!(f, "{}", s),
		}
	}
}

// a wrapper around f64 that implements Eq and Hash by comparing the bit representation of the float.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	PartialOrd,
	Deref,
	DerefMut,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct Float(pub f64);

impl Eq for Float {}

impl std::hash::Hash for Float {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.to_bits().hash(state);
	}
}

impl Ord for Float {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.0
			.partial_cmp(&other.0)
			.unwrap_or(std::cmp::Ordering::Equal)
	}
}

// cant From<()> as IntoBundle would conflict with (): Bundle

impl From<f64> for Float {
	fn from(value: f64) -> Self { Float(value) }
}

impl From<f32> for Float {
	fn from(value: f32) -> Self { Float(value as f64) }
}


impl From<bool> for Value {
	fn from(value: bool) -> Self { Value::Bool(value) }
}

impl From<i64> for Value {
	fn from(value: i64) -> Self { Value::Int(value) }
}

impl From<i32> for Value {
	fn from(value: i32) -> Self { Value::Int(value as i64) }
}

impl From<i16> for Value {
	fn from(value: i16) -> Self { Value::Int(value as i64) }
}

impl From<i8> for Value {
	fn from(value: i8) -> Self { Value::Int(value as i64) }
}

impl From<u64> for Value {
	fn from(value: u64) -> Self { Value::Uint(value) }
}

impl From<u32> for Value {
	fn from(value: u32) -> Self { Value::Uint(value as u64) }
}

impl From<u16> for Value {
	fn from(value: u16) -> Self { Value::Uint(value as u64) }
}

impl From<u8> for Value {
	fn from(value: u8) -> Self { Value::Uint(value as u64) }
}

impl From<f64> for Value {
	fn from(value: f64) -> Self { Value::Float(Float(value)) }
}

impl From<f32> for Value {
	fn from(value: f32) -> Self { Value::Float(Float(value as f64)) }
}

impl From<Float> for Value {
	fn from(value: Float) -> Self { Value::Float(value) }
}

impl From<String> for Value {
	fn from(value: String) -> Self { Value::Str(value) }
}

impl From<&str> for Value {
	fn from(value: &str) -> Self { Value::Str(value.to_string()) }
}


impl From<Vec<u8>> for Value {
	fn from(value: Vec<u8>) -> Self { Value::Bytes(value) }
}

impl From<&[u8]> for Value {
	fn from(value: &[u8]) -> Self { Value::Bytes(value.to_vec()) }
}
