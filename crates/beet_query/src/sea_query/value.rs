use std::char::TryFromCharError;
use std::convert::Infallible;
use std::num::TryFromIntError;

use sea_query::Value;

pub type ParseSeaValueResult<T> = Result<T, ParseSeaValueError>;

#[rustfmt::skip]
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParseSeaValueError {
	#[error("ParseSeaValue Conversion Failed: {error}")]
	ConversionFailed { error: String },
	#[error("ParseSeaValue Conversion Failed:\nExpected: {expected}\nReceived {received}")]
	TypeMismatch { expected: String, received: String },
	#[error("Unhandled Value Type:  {received_type}. `parse_sea_value` should be used in a way that guarantees the value type is handled")]
	UnhandledValueType { received_type: String },
}

impl ParseSeaValueError {
	pub fn conversion_failed(error: impl ToString) -> Self {
		Self::ConversionFailed {
			error: error.to_string(),
		}
	}
	pub fn type_mismatch<In, Out>() -> Self {
		Self::TypeMismatch {
			expected: std::any::type_name::<In>().to_string(),
			received: std::any::type_name::<Out>().to_string(),
		}
	}
	pub fn unhandled_value_type(value: &Value) -> Self {
		Self::UnhandledValueType {
			received_type: format!("{:?}", value),
		}
	}
}

macro_rules! into_conversion_error {
	($error_type:ty) => {
		impl From<$error_type> for ParseSeaValueError {
			fn from(error: $error_type) -> Self {
				Self::conversion_failed(error)
			}
		}
	};
}

into_conversion_error!(TryFromIntError);
into_conversion_error!(TryFromCharError);
into_conversion_error!(Infallible);

#[extend::ext(name=SeaQueryValueExt)]
pub impl Value {
	fn is_none(&self) -> bool {
		match self {
			| Value::Bool(None)
			| Value::TinyInt(None)
			| Value::SmallInt(None)
			| Value::Int(None)
			| Value::BigInt(None)
			| Value::TinyUnsigned(None)
			| Value::SmallUnsigned(None)
			| Value::Unsigned(None)
			| Value::BigUnsigned(None)
			| Value::Float(None)
			| Value::Double(None)
			| Value::String(None)
			| Value::Char(None)
			| Value::Bytes(None) => true,
			_ => false,
		}
	}
}


pub trait ParseSeaValue<T, M> {
	fn parse_sea_value(self) -> Result<T, ParseSeaValueError>;
}

pub struct OptionMarker;

impl ParseSeaValue<String, String> for Value {
	fn parse_sea_value(self) -> Result<String, ParseSeaValueError> {
		match self {
			Value::Bool(Some(val)) => Ok(val.to_string()),
			Value::TinyInt(Some(val)) => Ok(val.to_string()),
			Value::SmallInt(Some(val)) => Ok(val.to_string()),
			Value::Int(Some(val)) => Ok(val.to_string()),
			Value::BigInt(Some(val)) => Ok(val.to_string()),
			Value::TinyUnsigned(Some(val)) => Ok(val.to_string()),
			Value::SmallUnsigned(Some(val)) => Ok(val.to_string()),
			Value::Unsigned(Some(val)) => Ok(val.to_string()),
			Value::BigUnsigned(Some(val)) => Ok(val.to_string()),
			Value::Float(Some(val)) => Ok(val.to_string()), // lossy
			Value::Double(Some(val)) => Ok(val.to_string()), // lossy
			Value::String(Some(val)) => Ok(*val),
			Value::Char(Some(val)) => Ok(val.to_string()),
			Value::Bytes(Some(val)) => {
				Ok(String::from_utf8_lossy(&val).to_string())
			}
			value => Err(ParseSeaValueError::TypeMismatch {
				expected: std::any::type_name::<String>().to_string(),
				received: format!("{:?}", value),
			}),
		}
	}
}
impl<T, M> ParseSeaValue<Option<T>, (T, M, OptionMarker)> for Value
where
	Self: ParseSeaValue<T, M>,
{
	fn parse_sea_value(self) -> Result<Option<T>, ParseSeaValueError> {
		if self.is_none() {
			Ok(None)
		} else {
			Ok(Some(self.parse_sea_value()?))
		}
	}
}

macro_rules! impl_parse_sea_value_for_int {
	($t:ty) => {
		impl ParseSeaValue<$t, $t> for Value {
			fn parse_sea_value(self) -> Result<$t, ParseSeaValueError> {
				match self {
					Value::Bool(Some(val)) => Ok(if val { 1 } else { 0 }),
					Value::TinyInt(Some(val)) => Ok(val.try_into()?),
					Value::SmallInt(Some(val)) => Ok(val.try_into()?),
					Value::Int(Some(val)) => Ok(val.try_into()?),
					Value::BigInt(Some(val)) => Ok(val.try_into()?),
					Value::TinyUnsigned(Some(val)) => Ok(val.try_into()?),
					Value::SmallUnsigned(Some(val)) => Ok(val.try_into()?),
					Value::Unsigned(Some(val)) => Ok(val.try_into()?),
					Value::BigUnsigned(Some(val)) => Ok(val.try_into()?),
					Value::Float(Some(val)) => Ok(val as $t), // lossy
					Value::Double(Some(val)) => Ok(val as $t), // lossy
					Value::String(Some(val)) => val
						.parse()
						.map_err(|e| ParseSeaValueError::conversion_failed(e)),
					// Value::Char(Some(val)) => ,
					Value::Bytes(Some(val)) => String::from_utf8_lossy(&val)
						.parse()
						.map_err(|e| ParseSeaValueError::conversion_failed(e)),
					value => Err(ParseSeaValueError::TypeMismatch {
						expected: std::any::type_name::<$t>().to_string(),
						received: format!("{:?}", value),
					}),
				}
			}
		}
	};
}

impl_parse_sea_value_for_int!(u8);
impl_parse_sea_value_for_int!(u16);
impl_parse_sea_value_for_int!(u32);
impl_parse_sea_value_for_int!(u64);
impl_parse_sea_value_for_int!(u128);
impl_parse_sea_value_for_int!(usize);
impl_parse_sea_value_for_int!(i8);
impl_parse_sea_value_for_int!(i16);
impl_parse_sea_value_for_int!(i32);
impl_parse_sea_value_for_int!(i64);
impl_parse_sea_value_for_int!(i128);
impl_parse_sea_value_for_int!(isize);

macro_rules! impl_parse_sea_value_for_float {
	($t:ty) => {
		impl ParseSeaValue<$t, $t> for Value {
			fn parse_sea_value(self) -> Result<$t, ParseSeaValueError> {
				match self {
					Value::Bool(Some(val)) => Ok(if val { 1.0 } else { 0.0 }),
					Value::TinyInt(Some(val)) => Ok(val as $t),
					Value::SmallInt(Some(val)) => Ok(val as $t),
					Value::Int(Some(val)) => Ok(val as $t),
					Value::BigInt(Some(val)) => Ok(val as $t),
					Value::TinyUnsigned(Some(val)) => Ok(val as $t),
					Value::SmallUnsigned(Some(val)) => Ok(val as $t),
					Value::Unsigned(Some(val)) => Ok(val as $t),
					Value::BigUnsigned(Some(val)) => Ok(val as $t),
					Value::Float(Some(val)) => Ok(val as $t),
					Value::Double(Some(val)) => Ok(val as $t),
					Value::String(Some(val)) => val
						.parse()
						.map_err(|e| ParseSeaValueError::conversion_failed(e)),
					value => Err(ParseSeaValueError::TypeMismatch {
						expected: std::any::type_name::<$t>().to_string(),
						received: format!("{:?}", value),
					}),
				}
			}
		}
	};
}

impl_parse_sea_value_for_float!(f32);
impl_parse_sea_value_for_float!(f64);



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sea_query::Value;
	use sweet::prelude::*;

	#[test]
	fn basics() {
		Value::TinyUnsigned(Some(1))
			.parse_sea_value()
			.xpect()
			.to_be(Ok(1u8));
		Value::BigUnsigned(Some(u64::MAX))
			.xmap::<ParseSeaValueResult<u8>>(|val| val.parse_sea_value())
			.xpect()
			.to_be_err();
		Value::Bytes(None)
			.xmap::<ParseSeaValueResult<Option<u8>>>(|val| {
				val.parse_sea_value()
			})
			.xpect()
			.to_be(Ok(None));
	}
}
