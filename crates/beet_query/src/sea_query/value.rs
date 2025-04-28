use crate::prelude::*;
use sweet::prelude::*;


impl ConvertValue<sea_query::Value, sea_query::Value> for sea_query::Value {
	fn into_value(self) -> ConvertValueResult<Value> {
		match self {
			sea_query::Value::Bool(Some(val)) => val.into_value(),
			sea_query::Value::TinyInt(Some(val)) => val.into_value(),
			sea_query::Value::SmallInt(Some(val)) => val.into_value(),
			sea_query::Value::Int(Some(val)) => val.into_value(),
			sea_query::Value::BigInt(Some(val)) => val.into_value(),
			sea_query::Value::TinyUnsigned(Some(val)) => val.into_value(),
			sea_query::Value::SmallUnsigned(Some(val)) => val.into_value(),
			sea_query::Value::Unsigned(Some(val)) => val.into_value(),
			sea_query::Value::BigUnsigned(Some(val)) => val.into_value(),
			sea_query::Value::Float(Some(val)) => val.into_value(),
			sea_query::Value::Double(Some(val)) => val.into_value(),
			sea_query::Value::String(Some(val)) => (*val).into_value(),
			sea_query::Value::Char(Some(val)) => val.to_string().into_value(),
			sea_query::Value::Bytes(Some(val)) => (*val).into_value(),
			_ => ().into_value(),
		}
	}
	fn from_value(value: Value) -> ConvertValueResult<sea_query::Value> {
		match value {
			Value::Null => Ok(sea_query::Value::Int(None)),
			Value::Integer(val) => Ok(sea_query::Value::BigInt(Some(val))),
			Value::Real(val) => Ok(sea_query::Value::Double(Some(val))),
			Value::Text(val) => Ok(sea_query::Value::String(Some(val.into()))),
			Value::Blob(val) => Ok(sea_query::Value::Bytes(Some(val.into()))),
		}
	}
}


impl ConvertValue<sea_query::SimpleExpr, sea_query::Value>
	for sea_query::SimpleExpr
{
	fn into_value(self) -> ConvertValueResult<Value> {
		match self {
			sea_query::SimpleExpr::Value(val) => val.into_value(),
			other => Err(ConvertValueError::type_mismatch(
				"SimpleExpr::Value",
				other,
			)),
		}
	}
	fn from_value(value: Value) -> ConvertValueResult<sea_query::SimpleExpr> {
		sea_query::SimpleExpr::Value(value.into_other()?).xok()
	}
}
