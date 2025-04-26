use crate::prelude::*;

pub trait Table {
	type Columns: Columns;
	/// The name of the table in the database
	fn name() -> String;
	/// Whether an `IF NOT EXISTS` clause should be used when creating the table,
	/// defaults to `true`
	fn if_not_exists() -> bool { true }
}

///
pub trait Columns {
	type Table: Table;
	/// A list of all columns in the table
	fn all() -> Vec<Column>;
	fn into_column(&self) -> Column;
}


pub struct Column {
	/// The name of the column in the database
	pub name: String,
	/// The sql datatype
	///
	///  https://www.sqlite.org/datatype3.html
	pub value_type: ValueType,
	/// This translates to the inverse of the `NOT NULL` constraint
	pub optional: bool,
	/// This translates to the `DEFAULT` constraint
	pub default_value: Option<Value>,
	/// This translates to the `PRIMARY KEY` constraint
	pub primary_key: bool,
	/// This translates to the `AUTOINCREMENT` constraint.
	/// When building the table this should be ignored if the type
	/// is text.
	pub auto_increment: bool,
	/// This translates to the `UNIQUE` constraint
	pub unique: bool,
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
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Table)]
	#[table(name = "foobar")]
	#[allow(unused)]
	struct MyTable {
		#[allow(unused)]
		#[field(default = 9)]
		test: u32,
		optional: Option<String>,
	}


	#[test]
	fn works() {
		expect(MyTable::name()).to_be("foobar".to_string());
		expect(MyTableColumns::Test.into_column().name)
			.to_be("test".to_string());
	}
}
