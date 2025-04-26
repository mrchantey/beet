use std::str::FromStr;


pub type ParseValueResult<T> = std::result::Result<T, ParseValueError>;

#[derive(Debug, thiserror::Error)]
pub enum ParseValueError {
	#[error("Invalid value type {0}")]
	InvalidValue(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
	Null,
	Integer(i64),
	Real(f64),
	Text(String),
	Blob(Vec<u8>),
}

/// The possible types a column can be in libsql.
#[derive(Debug, Copy, Clone)]
pub enum ValueType {
	Integer = 1,
	Real,
	Text,
	Blob,
	Null,
}

impl ValueType {
	/// Returns the string representation of the value type.
	pub fn as_sql_str(&self) -> &'static str {
		match self {
			ValueType::Text => "TEXT",
			ValueType::Integer => "INTEGER",
			ValueType::Blob => "BLOB",
			ValueType::Null => "NULL",
			ValueType::Real => "REAL",
		}
	}
}

impl FromStr for ValueType {
	type Err = ();

	fn from_str(s: &str) -> std::result::Result<ValueType, Self::Err> {
		match s {
			"TEXT" => Ok(ValueType::Text),
			"INTEGER" => Ok(ValueType::Integer),
			"BLOB" => Ok(ValueType::Blob),
			"NULL" => Ok(ValueType::Null),
			"REAL" => Ok(ValueType::Real),
			_ => Err(()),
		}
	}
}

impl Value {
	/// Returns `true` if the value is [`Null`].
	///
	/// [`Null`]: Value::Null
	#[must_use]
	pub fn is_null(&self) -> bool { matches!(self, Self::Null) }

	/// Returns `true` if the value is [`Integer`].
	///
	/// [`Integer`]: Value::Integer
	#[must_use]
	pub fn is_integer(&self) -> bool { matches!(self, Self::Integer(..)) }

	/// Returns `true` if the value is [`Real`].
	///
	/// [`Real`]: Value::Real
	#[must_use]
	pub fn is_real(&self) -> bool { matches!(self, Self::Real(..)) }

	pub fn as_real(&self) -> Option<&f64> {
		if let Self::Real(v) = self {
			Some(v)
		} else {
			None
		}
	}

	/// Returns `true` if the value is [`Text`].
	///
	/// [`Text`]: Value::Text
	#[must_use]
	pub fn is_text(&self) -> bool { matches!(self, Self::Text(..)) }

	pub fn as_text(&self) -> Option<&String> {
		if let Self::Text(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub fn as_integer(&self) -> Option<&i64> {
		if let Self::Integer(v) = self {
			Some(v)
		} else {
			None
		}
	}

	/// Returns `true` if the value is [`Blob`].
	///
	/// [`Blob`]: Value::Blob
	#[must_use]
	pub fn is_blob(&self) -> bool { matches!(self, Self::Blob(..)) }

	pub fn as_blob(&self) -> Option<&Vec<u8>> {
		if let Self::Blob(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

pub trait TryIntoValue<M> {
	fn try_into_value(self) -> ParseValueResult<Value>;
}
impl<T: Into<Value>> TryIntoValue<T> for T {
	fn try_into_value(self) -> ParseValueResult<Value> { Ok(self.into()) }
}

// struct TryIntoValueMarker;
// impl<T: TryInto<Value>> TryIntoValue<TryIntoValueMarker> for T {
// 	fn try_into_value(self) -> ParseValueResult<Value> { Ok(self.try_into()?) }
// }

impl TryIntoValue<u64> for u64 {
	fn try_into_value(self) -> ParseValueResult<Value> {
		if self > i64::MAX as u64 {
			Err(ParseValueError::InvalidValue(format!(
				"Value {} is too large to fit in an i64",
				self
			)))
		} else {
			Ok(Value::Integer(self as i64))
		}
	}
}


impl From<i8> for Value {
	fn from(value: i8) -> Value { Value::Integer(value as i64) }
}

impl From<i16> for Value {
	fn from(value: i16) -> Value { Value::Integer(value as i64) }
}

impl From<i32> for Value {
	fn from(value: i32) -> Value { Value::Integer(value as i64) }
}

impl From<i64> for Value {
	fn from(value: i64) -> Value { Value::Integer(value) }
}

impl From<u8> for Value {
	fn from(value: u8) -> Value { Value::Integer(value as i64) }
}

impl From<u16> for Value {
	fn from(value: u16) -> Value { Value::Integer(value as i64) }
}

impl From<u32> for Value {
	fn from(value: u32) -> Value { Value::Integer(value as i64) }
}

impl TryFrom<u64> for Value {
	type Error = ParseValueError;

	fn try_from(value: u64) -> ParseValueResult<Value> {
		if value > i64::MAX as u64 {
			Err(ParseValueError::InvalidValue(format!(
				"Value {} is too large to fit in an i64",
				value
			)))
		} else {
			Ok(Value::Integer(value as i64))
		}
	}
}

impl From<f32> for Value {
	fn from(value: f32) -> Value { Value::Real(value as f64) }
}

impl From<f64> for Value {
	fn from(value: f64) -> Value { Value::Real(value) }
}

impl From<&str> for Value {
	fn from(value: &str) -> Value { Value::Text(value.to_owned()) }
}

impl From<String> for Value {
	fn from(value: String) -> Value { Value::Text(value) }
}

impl From<&[u8]> for Value {
	fn from(value: &[u8]) -> Value { Value::Blob(value.to_owned()) }
}

impl From<Vec<u8>> for Value {
	fn from(value: Vec<u8>) -> Value { Value::Blob(value) }
}

impl From<bool> for Value {
	fn from(value: bool) -> Value { Value::Integer(value as i64) }
}

impl<T> From<Option<T>> for Value
where
	T: Into<Value>,
{
	fn from(value: Option<T>) -> Self {
		match value {
			Some(inner) => inner.into(),
			None => Value::Null,
		}
	}
}


#[cfg(feature = "limbo")]
impl Into<limbo::Value> for Value {
	fn into(self) -> limbo::Value {
		match self {
			Value::Null => limbo::Value::Null,
			Value::Integer(i) => limbo::Value::Integer(i),
			Value::Real(r) => limbo::Value::Real(r),
			Value::Text(t) => limbo::Value::Text(t),
			Value::Blob(b) => limbo::Value::Blob(b),
		}
	}
}

#[cfg(feature = "limbo")]
impl From<limbo::Value> for Value {
	fn from(value: limbo::Value) -> Self {
		match value {
			limbo::Value::Null => Value::Null,
			limbo::Value::Integer(i) => Value::Integer(i),
			limbo::Value::Real(r) => Value::Real(r),
			limbo::Value::Text(t) => Value::Text(t),
			limbo::Value::Blob(b) => Value::Blob(b),
		}
	}
}
