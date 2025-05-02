use crate::prelude::*;

pub type ConvertValueResult<T> = Result<T, ConvertValueError>;

#[rustfmt::skip]
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ConvertValueError {
	#[error("ConvertValue Failed: {error}")]
	ConversionFailed { error: String },
	#[error("Expected: {expected}\n Received: {received}")]
	TypeMismatch { expected: String,received: String },
}

impl ConvertValueError {
	pub fn conversion_failed(error: impl ToString) -> Self {
		Self::ConversionFailed {
			error: error.to_string(),
		}
	}
	pub fn type_mismatch(
		expected: impl ToString,
		received: impl std::fmt::Debug,
	) -> Self {
		Self::TypeMismatch {
			expected: expected.to_string(),
			received: format!("{:?}", received),
		}
	}
}

/// Sqlite types are the lowest common denominator of all sql types
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
	Null,
	Integer(i64),
	Real(f64),
	Text(String),
	Blob(Vec<u8>),
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Null => write!(f, "NULL"),
			Value::Integer(val) => write!(f, "{}", val),
			Value::Real(val) => write!(f, "{}", val),
			Value::Text(val) => write!(f, "'{}'", val),
			Value::Blob(val) => {
				write!(
					f,
					"[{}]",
					val.iter()
						.map(|v| v.to_string())
						.collect::<Vec<_>>()
						.join(", ")
				)
			}
		}
	}
}

impl Value {}
/// Convert a [`Value`] into another type by specifying the type.
pub trait ValueIntoOther {
	/// Convert a [`Value`] into another type that implements [`ConvertValue`],
	/// for example [`sea_query::Value`] or [`libsql::Value`]
	fn into_other<T>(self) -> ConvertValueResult<T>
	where
		T: ConvertValue;
}

impl ValueIntoOther for Value {
	fn into_other<T>(self) -> ConvertValueResult<T>
	where
		T: ConvertValue,
	{
		T::from_value(self)
	}
}
impl ValueIntoValueType for Value {
	fn value_type(&self) -> ValueType {
		match self {
			Value::Null => ValueType::Null,
			Value::Integer(_) => ValueType::Integer,
			Value::Real(_) => ValueType::Real,
			Value::Text(_) => ValueType::Text,
			Value::Blob(_) => ValueType::Blob,
		}
	}
}

pub trait ConvertValue: Sized {
	fn into_value(self) -> ConvertValueResult<Value>;
	fn from_value(value: Value) -> ConvertValueResult<Self>;
}
impl ConvertValue for () {
	fn into_value(self) -> ConvertValueResult<Value> { Ok(Value::Null) }
	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Null => Ok(()),
			_ => Err(ConvertValueError::conversion_failed(format!(
				"Expected Value::Null, found {:?}",
				value
			))),
		}
	}
}

impl ConvertValue for String {
	fn into_value(self) -> ConvertValueResult<Value> { Ok(Value::Text(self)) }
	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Text(val) => Ok(val),
			Value::Blob(val) => Ok(String::from_utf8_lossy(&val).to_string()),
			_ => Err(ConvertValueError::conversion_failed(format!(
				"Expected Value::Text, found {:?}",
				value
			))),
		}
	}
}

impl ConvertValue for bool {
	fn into_value(self) -> ConvertValueResult<Value> {
		Ok(Value::Integer(if self { 1 } else { 0 }))
	}

	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Integer(val) => Ok(val != 0),
			_ => Err(ConvertValueError::conversion_failed(format!(
				"Expected Value::Integer, found {:?}",
				value
			))),
		}
	}
}

impl ConvertValue for Vec<u8> {
	fn into_value(self) -> ConvertValueResult<Value> { Ok(Value::Blob(self)) }
	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Blob(val) => Ok(val),
			Value::Text(val) => Ok(val.into_bytes()),
			_ => Err(ConvertValueError::conversion_failed(format!(
				"Expected Value::Blob, found {:?}",
				value
			))),
		}
	}
}

// Generic Option<T> ConvertValue implementation
impl<T: ConvertValue> ConvertValue for Option<T> {
	fn into_value(self) -> ConvertValueResult<Value> {
		match self {
			Some(value) => value.into_value(),
			None => Ok(Value::Null),
		}
	}
	fn from_value(value: Value) -> ConvertValueResult<Self> {
		if value == Value::Null {
			Ok(None)
		} else {
			T::from_value(value).map(Some)
		}
	}
}

// TryInto<i64> ConvertValue implementation using macro_rules!
macro_rules! impl_convert_value_try_into_i64 {
    ($($t:ty),*) => {
        $(
            impl ConvertValue for $t {
                fn into_value(self) -> ConvertValueResult<Value> {
                    match self.try_into() {
                        Ok(value) => Ok(Value::Integer(value)),
                        Err(err) => Err(ConvertValueError::conversion_failed(format!(
                            "Failed to convert {} to i64: {:?}",
                            std::any::type_name::<$t>(),
                            err,
                        ))),
                    }
                }
                fn from_value(value: Value) -> ConvertValueResult<Self> {
                    match value {
                        Value::Integer(val) => match <$t>::try_from(val) {
                            Ok(value) => Ok(value),
                            Err(err) => Err(ConvertValueError::conversion_failed(format!(
                                "Failed to convert i64 to {}: {:?}",
                                std::any::type_name::<$t>(),
                                err,
                            ))),
                        },
                        _ => Err(ConvertValueError::conversion_failed(format!(
                            "Expected Value::Integer, found {:?}",
                            value
                        ))),
                    }
                }
            }
        )*
    };
}

impl_convert_value_try_into_i64!(
	u8, i8, u16, i16, u32, i32, u64, i64, usize, isize
);

impl ConvertValue for f32 {
	fn into_value(self) -> ConvertValueResult<Value> {
		Ok(Value::Real(self as f64))
	}

	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Real(val) => Ok(val as f32),
			_ => Err(ConvertValueError::conversion_failed(format!(
				"Expected Value::Real, found {:?}",
				value
			))),
		}
	}
}

impl ConvertValue for f64 {
	fn into_value(self) -> ConvertValueResult<Value> { Ok(Value::Real(self)) }

	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Real(val) => Ok(val),
			_ => Err(ConvertValueError::conversion_failed(format!(
				"Expected Value::Real, found {:?}",
				value
			))),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::types::value::ConvertValue;
	use std::str::FromStr;
	use sweet::prelude::*; // Add explicit import

	#[test]
	fn test_value_conversions_integer() {
		// Test u8 conversion
		expect(3u8.into_value()).to_be(Ok(Value::Integer(3)));
		expect(255u8.into_value()).to_be(Ok(Value::Integer(255)));

		// Test i8 conversion
		expect((-5i8).into_value()).to_be(Ok(Value::Integer(-5)));

		// Test u16/i16 conversion
		expect(1000u16.into_value()).to_be(Ok(Value::Integer(1000)));
		expect((-1000i16).into_value()).to_be(Ok(Value::Integer(-1000)));

		// Test u32/i32 conversion
		expect(100000u32.into_value()).to_be(Ok(Value::Integer(100000)));
		expect((-100000i32).into_value()).to_be(Ok(Value::Integer(-100000)));

		// Test u64 conversion - should fail if value exceeds i64::MAX
		expect(u64::MAX.into_value()).to_be_err();
		expect(42u64.into_value()).to_be(Ok(Value::Integer(42)));

		// Test i64 conversion
		expect(i64::MAX.into_value()).to_be(Ok(Value::Integer(i64::MAX)));
		expect(i64::MIN.into_value()).to_be(Ok(Value::Integer(i64::MIN)));

		// Test usize conversion
		expect(100usize.into_value()).to_be(Ok(Value::Integer(100)));

		// Test isize conversion
		expect((-100isize).into_value()).to_be(Ok(Value::Integer(-100)));
	}

	#[test]
	fn test_value_conversions_real() {
		// Test f32 conversion
		expect(3.14f32.into_value()).to_be(Ok(Value::Real(3.140000104904175)));
		expect((-42.5f32).into_value()).to_be(Ok(Value::Real(-42.5f64)));

		// Test f64 conversion
		expect(3.14159265359f64.into_value())
			.to_be(Ok(Value::Real(3.14159265359)));
		expect(f64::MAX.into_value()).to_be(Ok(Value::Real(f64::MAX)));
		expect(f64::MIN.into_value()).to_be(Ok(Value::Real(f64::MIN)));
	}

	#[test]
	fn test_value_conversions_text() {
		// Test String conversion
		let s = "Hello, world!".to_string();
		expect(s.clone().into_value()).to_be(Ok(Value::Text(s)));

		// Empty string
		expect("".to_string().into_value())
			.to_be(Ok(Value::Text("".to_string())));

		// Unicode
		let unicode = "こんにちは世界".to_string();
		expect(unicode.clone().into_value()).to_be(Ok(Value::Text(unicode)));
	}

	#[test]
	fn test_value_conversions_blob() {
		// Test Vec<u8> conversion
		let bytes = vec![1, 2, 3, 4, 5];
		expect(bytes.clone().into_value()).to_be(Ok(Value::Blob(bytes)));

		// Empty blob
		expect(Vec::<u8>::new().into_value())
			.to_be(Ok(Value::Blob(Vec::new())));
	}

	#[test]
	fn test_value_option_type() {
		// Some values
		expect(Some(42i64).into_value()).to_be(Ok(Value::Integer(42)));
		expect(Some("hello".to_string()).into_value())
			.to_be(Ok(Value::Text("hello".to_string())));

		// None values
		expect(Option::<i64>::None.into_value()).to_be(Ok(Value::Null));
		expect(Option::<String>::None.into_value()).to_be(Ok(Value::Null));

		// Converting back
		expect(Option::<i64>::from_value(Value::Null)).to_be(Ok(None));
		expect(Option::<i64>::from_value(Value::Integer(42)))
			.to_be(Ok(Some(42)));
		expect(Option::<String>::from_value(Value::Text(
			"hello".to_string(),
		)))
		.to_be(Ok(Some("hello".to_string())));
	}

	#[test]
	fn test_from_value() {
		// Integer conversions
		expect(u8::from_value(Value::Integer(42))).to_be(Ok(42u8));
		expect(i8::from_value(Value::Integer(-42))).to_be(Ok(-42i8));
		expect(u16::from_value(Value::Integer(1000))).to_be(Ok(1000u16));
		expect(i16::from_value(Value::Integer(-1000))).to_be(Ok(-1000i16));
		expect(u32::from_value(Value::Integer(100000))).to_be(Ok(100000u32));
		expect(i32::from_value(Value::Integer(-100000))).to_be(Ok(-100000i32));
		expect(u64::from_value(Value::Integer(42))).to_be(Ok(42u64));
		expect(i64::from_value(Value::Integer(-42))).to_be(Ok(-42i64));

		// Out of range conversions should fail
		expect(u8::from_value(Value::Integer(256))).to_be_err();
		expect(u8::from_value(Value::Integer(-1))).to_be_err();
		expect(i8::from_value(Value::Integer(128))).to_be_err();
		expect(i8::from_value(Value::Integer(-129))).to_be_err();

		// Wrong type conversions should fail
		expect(u8::from_value(Value::Real(42.0))).to_be_err();
		expect(i32::from_value(Value::Text("42".to_string()))).to_be_err();

		// Real conversions
		expect(f32::from_value(Value::Real(3.14))).to_be(Ok(3.14f32));
		expect(f64::from_value(Value::Real(3.14159265359)))
			.to_be(Ok(3.14159265359f64));

		// Text conversions
		expect(String::from_value(Value::Text("hello".to_string())))
			.to_be(Ok("hello".to_string()));

		// Blob conversions
		let bytes = vec![1, 2, 3, 4, 5];
		expect(Vec::<u8>::from_value(Value::Blob(bytes.clone())))
			.to_be(Ok(bytes));

		// Cross-type conversions
		expect(String::from_value(Value::Blob(vec![
			72, 101, 108, 108, 111,
		])))
		.to_be(Ok("Hello".to_string()));
		expect(Vec::<u8>::from_value(Value::Text("Hello".to_string())))
			.to_be(Ok(vec![72, 101, 108, 108, 111]));
	}

	#[test]
	fn test_value_type() {
		expect(Value::Null.value_type() as u8).to_be(ValueType::Null as u8);
		expect(Value::Integer(42).value_type() as u8)
			.to_be(ValueType::Integer as u8);
		expect(Value::Real(3.14).value_type() as u8)
			.to_be(ValueType::Real as u8);
		expect(Value::Text("hello".to_string()).value_type() as u8)
			.to_be(ValueType::Text as u8);
		expect(Value::Blob(vec![1, 2, 3]).value_type() as u8)
			.to_be(ValueType::Blob as u8);
	}

	#[test]
	fn test_value_type_from_str() {
		expect(ValueType::from_str("INTEGER")).to_be_ok_val(ValueType::Integer);
		expect(ValueType::from_str("REAL")).to_be_ok_val(ValueType::Real);
		expect(ValueType::from_str("TEXT")).to_be_ok_val(ValueType::Text);
		expect(ValueType::from_str("BLOB")).to_be_ok_val(ValueType::Blob);
		expect(ValueType::from_str("NULL")).to_be_ok_val(ValueType::Null);

		// Invalid types should error
		expect(ValueType::from_str("INVALID").is_err()).to_be(true);
		expect(ValueType::from_str("").is_err()).to_be(true);
		expect(ValueType::from_str("integer").is_err()).to_be(true); // Case sensitive
	}

	#[test]
	fn test_conversion_error() {
		let error = ConvertValueError::conversion_failed("test error");
		match error {
			ConvertValueError::ConversionFailed { error } => {
				expect(error).to_be("test error".to_string());
			}
			_ => panic!("Expected ConversionFailed error"),
		}

		// Test error message format
		let error_str =
			format!("{}", ConvertValueError::conversion_failed("test error"));
		expect(error_str).to_be("ConvertValue Failed: test error".to_string());
	}
}
