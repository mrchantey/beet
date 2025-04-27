use crate::prelude::*;
use anyhow::Result;
use sea_query::ColumnDef;
use sea_query::Iden;
use sea_query::InsertStatement;
use sea_query::Query;
use sea_query::TableCreateStatement;
use sea_query::Values;
use std::borrow::Cow;
use sweet::prelude::*;

pub trait Table {
	/// An enum representing all columns in the table
	type Columns: 'static + Iden + Columns;
	/// The name of the table in the database
	fn name() -> Cow<'static, str>;
	/// Generate a create statement for the table,
	/// usually with options defined in `#[derive(Table)]`
	fn stmt_create_table() -> TableCreateStatement;

	/// Execute a `CREATE TABLE` statement with this table's [`stmt_create_table`](Table::stmt_create_table)
	async fn create_table(conn: &impl Connection) -> Result<()> {
		conn.execute(Self::stmt_create_table()).await
	}
}

/// A trait for a list of columns in a table
pub trait Columns: Sized + IntoColumnDef {
	/// The table these columns are for
	type Table: Table;
	/// Returns a list of all columns in the table
	fn variants() -> Vec<Self>;

	/// A list of all columns in the table
	fn column_defs() -> Vec<ColumnDef> {
		Self::variants()
			.into_iter()
			.map(|col| col.into_column_def())
			.collect()
	}
}

/// A trait for a partial view of a table,
/// used for insert, update and query statements
pub trait TableView: Sized {
	/// The table this view is for
	type Table: Table;
	/// All columns for this insert view. This includes Optional columns,
	/// which will be set to ? in the insert statement if `None`
	fn columns() -> Vec<<Self::Table as Table>::Columns>;
	/// Converts the view into a list of [`SimpleExpr`]
	/// for an insert or update statement
	fn into_values(self) -> Values;

	fn stmt_select_all() -> sea_query::SelectStatement {
		Query::select()
			.from(CowIden(Self::Table::name()))
			.columns(Self::columns())
			.to_owned()
	}


	fn stmt_insert(self) -> Result<InsertStatement> {
		Query::insert()
			.into_table(CowIden(Self::Table::name()))
			.columns(Self::columns())
			.values(self.into_values().0.into_iter().map(|v| v.into()))?
			.to_owned()
			.xok()
	}

	async fn insert(self, conn: &impl Connection) -> Result<()> {
		conn.execute(self.stmt_insert()?).await
	}
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
