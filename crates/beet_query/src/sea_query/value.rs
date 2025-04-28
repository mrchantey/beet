use sea_query::Value;

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParseSeaValueError {
	#[error(
		"ParseSeaValue Conversion Failed:\nExpected: {expected}\nReceived {received}"
	)]
	ConversionError { expected: String, received: String },
	#[error(
		"Unhandled Value Type:  {received_type}. `parse_sea_value` should be used in a way that guarantees the value type is handled"
	)]
	UnhandledValueType { received_type: String },
}

impl ParseSeaValueError {
	pub fn conversion_error<In, Out>() -> Self {
		Self::ConversionError {
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

pub trait ParseSeaValue<T, M> {
	fn parse_sea_value(self) -> Result<T, ParseSeaValueError>;
}

pub struct OptionMarker;

impl ParseSeaValue<u8, u8> for Value {
	fn parse_sea_value(self) -> Result<u8, ParseSeaValueError> {
		match self {
			Value::TinyUnsigned(Some(val)) => Ok(val),
			value => Err(ParseSeaValueError::ConversionError {
				expected: std::any::type_name::<u8>().to_string(),
				received: format!("{:?}", value),
			}),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sea_query::Value;
	use sweet::prelude::*;

	#[test]
	fn u8() {
		Value::TinyUnsigned(Some(1))
			.parse_sea_value()
			.xpect()
			.to_be(Ok(1u8));

		// same null type
		// try_parse_sea_value::<Option<f32>>(Value::Float(None))
		// 	.xpect()
		// 	.to_be(Ok(None));
		// // different null types allowed
		// try_parse_sea_value::<Option<f32>>(Value::Int(None))
		// 	.xpect()
		// 	.to_be(Ok(None));
	}

	// #[test]
	// fn fuzzy() {
}
