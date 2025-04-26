mod sqlite;
use crate::prelude::*;
use anyhow::Result;
pub use sqlite::*;

pub trait QueryBuilder {
	/// Returns a statement to initialize the database
	fn init<T: Table>() -> Result<String>;
	fn drop<T: Table>() -> Result<String>;
	fn prepare_insert<T: TableView>() -> Result<String>;
	fn execute_insert<T: TableView>(view: T) -> Result<String>;
}
