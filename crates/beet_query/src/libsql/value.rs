use crate::prelude::*;


impl ConvertValue for libsql::Value {
	fn into_value(self) -> ConvertValueResult<Value> {
		match self {
			libsql::Value::Null => Ok(Value::Null),
			libsql::Value::Integer(i) => Ok(Value::Integer(i)),
			libsql::Value::Real(r) => Ok(Value::Real(r)),
			libsql::Value::Text(t) => Ok(Value::Text(t)),
			libsql::Value::Blob(b) => Ok(Value::Blob(b)),
		}
	}
	fn from_value(value: Value) -> ConvertValueResult<libsql::Value> {
		match value {
			Value::Null => Ok(libsql::Value::Null),
			Value::Integer(i) => Ok(libsql::Value::Integer(i)),
			Value::Real(r) => Ok(libsql::Value::Real(r)),
			Value::Text(t) => Ok(libsql::Value::Text(t)),
			Value::Blob(b) => Ok(libsql::Value::Blob(b)),
		}
	}
}
