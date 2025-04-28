#[extend::ext(name=SeaQueryValuesToLibsql)]
pub impl sea_query::Values {
	fn into_libsql_values(self) -> Vec<libsql::Value> {
		self.0
			.into_iter()
			.map(|value| value.into_libsql_value())
			.collect()
	}
}



#[extend::ext(name=SeaQueryValueToLibsql)]
pub impl sea_query::Value {
	/// Converts a [`sea_query::Value`] into a [`libsql::Value`].
	/// Libsql values are sqlite values:
	/// ```no_run
	/// Null,
	/// Integer(i64),
	/// Real(f64),
	/// Text(String),
	/// Blob(Vec<u8>),
	/// ```
	fn into_libsql_value(self) -> libsql::Value {
		match self {
			sea_query::Value::Bool(Some(val)) => match val {
				true => libsql::Value::Integer(1),
				false => libsql::Value::Integer(0),
			},
			sea_query::Value::TinyInt(Some(val)) => {
				libsql::Value::Integer(val as i64)
			}
			sea_query::Value::SmallInt(Some(val)) => {
				libsql::Value::Integer(val as i64)
			}
			sea_query::Value::Int(Some(val)) => {
				libsql::Value::Integer(val as i64)
			}
			sea_query::Value::BigInt(Some(val)) => libsql::Value::Integer(val),
			sea_query::Value::TinyUnsigned(Some(val)) => {
				libsql::Value::Integer(val as i64)
			}
			sea_query::Value::SmallUnsigned(Some(val)) => {
				libsql::Value::Integer(val as i64)
			}
			sea_query::Value::Unsigned(Some(val)) => {
				libsql::Value::Integer(val as i64)
			}
			sea_query::Value::BigUnsigned(Some(val)) => {
				if let Ok(val) = i64::try_from(val) {
					libsql::Value::Integer(val)
				} else {
					// Fallback for values that don't fit in i64
					libsql::Value::Real(val as f64)
				}
			}
			sea_query::Value::Float(Some(val)) => {
				libsql::Value::Real(val as f64)
			}
			sea_query::Value::Double(Some(val)) => libsql::Value::Real(val),
			sea_query::Value::String(Some(val)) => libsql::Value::Text(*val),
			sea_query::Value::Char(Some(val)) => {
				libsql::Value::Text(val.to_string())
			}
			sea_query::Value::Bytes(Some(items)) => libsql::Value::Blob(*items),
			// for all None option types
			_ => libsql::Value::Null,
		}
	}
}


#[extend::ext(name=LibsqlValueToSeaQuery)]
pub impl libsql::Value {
	#[rustfmt::skip]
	fn into_sea_query_value(self) -> sea_query::Value {
		match self {
			// no null, i guess int is fine?
			Self::Null => sea_query::Value::Int(None),
			Self::Integer(i) => sea_query::Value::BigInt(Some(i)),
			Self::Real(f) => sea_query::Value::Double(Some(f)),
			Self::Text(text) => sea_query::Value::String(Some(Box::new(text))),
			Self::Blob(items) => sea_query::Value::Bytes(Some(Box::new(items))),
		}
	}
}
