use crate::prelude::*;


impl ConvertValue for limbo::Value {
	fn into_value(self) -> ConvertValueResult<Value> {
		match self {
			limbo::Value::Null => Ok(Value::Null),
			limbo::Value::Integer(i) => Ok(Value::Integer(i)),
			limbo::Value::Real(r) => Ok(Value::Real(r)),
			limbo::Value::Text(t) => Ok(Value::Text(t)),
			limbo::Value::Blob(b) => Ok(Value::Blob(b)),
		}
	}
	fn from_value(value: Value) -> ConvertValueResult<limbo::Value> {
		match value {
			Value::Null => Ok(limbo::Value::Null),
			Value::Integer(i) => Ok(limbo::Value::Integer(i)),
			Value::Real(r) => Ok(limbo::Value::Real(r)),
			Value::Text(t) => Ok(limbo::Value::Text(t)),
			Value::Blob(b) => Ok(limbo::Value::Blob(b)),
		}
	}
}
