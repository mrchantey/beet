use crate::prelude::*;
use anyhow::Result;
use sea_query::ColumnDef;
use sea_query::TableCreateStatement;
use std::borrow::Cow;

pub trait Table {
	/// An enum representing all columns in the table
	type Columns: Columns;
	/// The name of the table in the database
	fn name() -> Cow<'static, str>;
	/// Generate a create statement for the table,
	/// usually with options defined in `#[derive(Table)]`
	fn stmt_create_table() -> TableCreateStatement;


	async fn create_table(conn: &impl Connection) -> Result<()> {
		conn.execute(Self::stmt_create_table()).await
	}
}

/// A trait for a list of columns in a table
pub trait Columns {
	/// The table these columns are for
	type Table: Table;
	/// A list of all columns in the table
	fn all() -> Vec<ColumnDef>;
}

/// A trait for a partial view of a table,
/// used for insert, update and query statements
pub trait TableView {
	/// The table this view is for
	type Table: Table;
	/// All columns for this insert view. This includes Optional columns,
	/// which will be set to ? in the insert statement if `None`
	fn columns() -> Vec<<Self::Table as Table>::Columns>;
	/// Converts the view into a list of values
	/// for an insert or update statement
	fn into_values(self) -> ParseValueResult<Vec<Value>>;
}


#[cfg(test)]
mod test {
	// use crate::as_beet::*;
	// use sweet::prelude::*;

	// #[derive(Table)]
	// #[table(name = "foobar")]
	// #[allow(unused)]
	// struct MyTable {
	// 	#[allow(unused)]
	// 	#[field(default = 9)]
	// 	test: u32,
	// 	optional: Option<String>,
	// }


	// struct User {
	// 	id: u32,
	// 	name: String,
	// }
	// 	pub struct UserId(pub u32);
	// 	pub struct UserEmail(pub String);


	#[test]
	fn works() {
		// expect(MyTable::name()).to_be("foobar".to_string());
		// expect(MyTableColumns::Test.into_column().name)
		// 	.to_be("test".to_string());
	}
}
