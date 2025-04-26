pub trait Table {
	type Columns: Columns;
	/// The name of the table in the database
	fn name() -> &'static str;
}

///
pub trait Columns {
	fn columns() -> Vec<String>;
}
