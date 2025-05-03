use crate::prelude::*;
use sweet::prelude::*;


impl ConvertValue for sea_query::Value {
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
	fn from_value(value: Value) -> ConvertValueResult<Self> {
		match value {
			Value::Null => Ok(sea_query::Value::Int(None)),
			Value::Integer(val) => Ok(sea_query::Value::BigInt(Some(val))),
			Value::Real(val) => Ok(sea_query::Value::Double(Some(val))),
			Value::Text(val) => Ok(sea_query::Value::String(Some(val.into()))),
			Value::Blob(val) => Ok(sea_query::Value::Bytes(Some(val.into()))),
		}
	}
}


impl ConvertValue for sea_query::SimpleExpr {
	fn into_value(self) -> ConvertValueResult<Value> {
		match self {
			sea_query::SimpleExpr::Value(val) => val.into_value(),
			other => Err(ConvertValueError::type_mismatch(
				"SimpleExpr::Value",
				other,
			)),
		}
	}
	fn from_value(value: Value) -> ConvertValueResult<Self> {
		sea_query::SimpleExpr::Value(value.into_other()?).xok()
	}
}


impl ValueType {
	/// Converts a [`ValueType`] to a [`sea_query::ColumnType`].
	pub fn into_column_type(self) -> sea_query::ColumnType {
		match self {
			ValueType::Integer => sea_query::ColumnType::BigInteger,
			ValueType::Real => sea_query::ColumnType::Double,
			ValueType::Text => sea_query::ColumnType::Text,
			ValueType::Blob => sea_query::ColumnType::Blob,
			ValueType::Null => sea_query::ColumnType::BigInteger,
		}
	}
}


#[extend::ext(name=SeaQueryValuesExt)]
pub impl sea_query::Values {
	fn into_row(self) -> ConvertValueResult<Row> {
		Row::new(
			self.into_iter()
				.map(|v| v.into_value())
				.collect::<ConvertValueResult<Vec<_>>>()?,
		)
		.xok()
	}
}
