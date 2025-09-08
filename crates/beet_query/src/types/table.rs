use crate::prelude::*;
use sea_query::ColumnDef;
use sea_query::Iden;
use sea_query::TableCreateStatement;
use std::borrow::Cow;

/// TODO all [sea_query::SchemaStatement]
pub trait Table {
	/// An enum representing all columns in the table
	type Columns: 'static + Iden + Columns;
	/// The name of the table in the database
	fn name() -> Cow<'static, str>;
	/// Generate a create statement for the table,
	/// usually with options defined in `#[derive(Table)]`
	fn stmt_create_table() -> TableCreateStatement;
}

/// A trait for a list of columns in a table
pub trait Columns: Sized + IntoColumnDef + ValueIntoValueType {
	/// The table these columns are for
	type Table: Table;
	/// Returns a list of all columns in the table
	fn variants() -> Vec<Self>;

	/// Returns the column that is the primary key for this table
	fn primary_key() -> Option<Self> { None }

	/// A list of all columns in the table
	fn column_defs() -> Vec<ColumnDef> {
		Self::variants()
			.into_iter()
			.map(|col| col.into_column_def())
			.collect()
	}

	/// Returns the type of the primary key column, or `ValueType::Null` if there is no primary key
	fn primary_key_value_type() -> ValueType {
		Self::primary_key()
			.map(|col| col.value_type())
			.unwrap_or(ValueType::Null)
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	// #[derive(Table)]
	// #[table(name = "foobar")]
	// #[allow(unused)]
	// struct MyTable {
	// 	#[allow(unused)]
	// 	#[field(default = 9)]
	// 	test: u32,
	// 	optional: Option<String>,
	// }

	#[derive(Default, Table)]
	struct User {
		id: u32,
		name: String,
	}
	// 	pub struct UserId(pub u32);
	// 	pub struct UserEmail(pub String);


	#[test]
	fn works() {
		UserCols::primary_key().xpect().to_be(Some(UserCols::Id));
		User::default()
			.primary_value()
			.unwrap()
			.xpect()
			.to_be(Some(0u32.into_value().unwrap()));

		// MyTable::name().xpect().to_be("foobar".to_string());
		// MyTableColumns::Test.into_column().name
		// 	.xpect()
		// 	.to_be("test".to_string());
	}
}
