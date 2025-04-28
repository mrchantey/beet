use std::str::FromStr;



/// Sqlite types are the lowest common denominator of all the types
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ValueType {
	/// Starting from 1 to match sqlite
	Integer = 1,
	Real,
	Text,
	Blob,
	Null,
}

pub trait TypeIntoValueType: Sized {
	fn value_type() -> ValueType;
}

/// From a value, get its [`ValueType`]. This is implemented
/// for [`Value`] and types that implement [`Columns`].
pub trait ValueIntoValueType: Sized {
	fn value_type(&self) -> ValueType;
}

impl std::fmt::Display for ValueType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			ValueType::Integer => "INTEGER",
			ValueType::Real => "REAL",
			ValueType::Text => "TEXT",
			ValueType::Blob => "BLOB",
			ValueType::Null => "NULL",
		})
	}
}


macro_rules! impl_value_type {
	($($t:ty => $value_type:expr),* $(,)?) => {
		$(
			impl TypeIntoValueType for $t {
				fn value_type() -> ValueType { $value_type }
			}
		)*
	};
}

impl_value_type! {
	u8 => ValueType::Integer,
	u16 => ValueType::Integer,
	u32 => ValueType::Integer,
	u64 => ValueType::Integer,
	i8 => ValueType::Integer,
	i16 => ValueType::Integer,
	i32 => ValueType::Integer,
	i64 => ValueType::Integer,
	usize => ValueType::Integer,
	isize => ValueType::Integer,
	f32 => ValueType::Real,
	f64 => ValueType::Real,
	String => ValueType::Text,
	Vec<u8> => ValueType::Blob,
	// str=> ValueType::Text,
}


impl FromStr for ValueType {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> std::result::Result<ValueType, Self::Err> {
		match s {
			"TEXT" => Ok(ValueType::Text),
			"INTEGER" => Ok(ValueType::Integer),
			"BLOB" => Ok(ValueType::Blob),
			"NULL" => Ok(ValueType::Null),
			"REAL" => Ok(ValueType::Real),
			_ => anyhow::bail!("Invalid ValueType: {}", s),
		}
	}
}
