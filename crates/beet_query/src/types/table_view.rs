use crate::prelude::*;
use anyhow::Result;
use sea_query::Expr;
use sea_query::InsertStatement;
use sea_query::Query;
use sea_query::Value;
use sea_query::Values;
use sweet::prelude::*;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DeserializeError {
	#[error("Row Length Mismatch: expected: {expected}, received: {received}")]
	RowLengthMismatch { expected: usize, received: usize },
	#[error("Type Mismatch: expected: {expected}, received: {received}")]
	TypeMismatch { expected: String, received: String },
}

/// A trait for a partial view of a table,
/// used for insert, update and query statements
/// TODO all [sea_query::QueryStatement]
pub trait TableView: Sized {
	/// The table this view is for
	type Table: Table;
	/// All columns for this insert view. This includes Optional columns,
	/// which will be set to ? in the insert statement if `None`.
	/// This must be the same length and order as [`Self::into_values`](TableView::into_values)
	fn columns() -> Vec<<Self::Table as Table>::Columns>;
	/// Like [`serde::Serialize`], converts the view into a [`Values`]
	/// for an insert or update statement.
	/// This must be the same length and order as [`Self::columns`](TableView::columns)
	fn into_values(self) -> Values;

	/// Like [`serde::Deserialize`], converts a row of values into this view.
	fn from_values(values: Vec<Value>) -> Result<Self, DeserializeError>;
	/// Returns the value of the primary key for this table, its type
	/// should match [`Self::Table::Columns::primary_key`](Columns::primary_key)
	fn primary_value(&self) -> Option<Value> { None }

	/// Returns the primary key and value for this table if both exist
	fn primary_kvp(&self) -> Result<(<Self::Table as Table>::Columns, Value)> {
		let key = <Self::Table as Table>::Columns::primary_key().ok_or_else(
			|| {
				anyhow::anyhow!(
					"No primary key defined for table {}",
					Self::Table::name()
				)
			},
		)?;

		let value = self.primary_value().ok_or_else(|| {
			anyhow::anyhow!(
				"No primary key value provided for {}",
				std::any::type_name::<Self>(),
			)
		})?;

		Ok((key, value))
	}


	/// Create an insert statement for this table
	/// with all columns and values in this view
	fn stmt_insert(self) -> Result<InsertStatement> {
		Query::insert()
			.into_table(CowIden(Self::Table::name()))
			.columns(Self::columns())
			.values(self.into_values().0.into_iter().map(|v| v.into()))?
			.to_owned()
			.xok()
	}

	/// Create a select statement for this table
	/// with all columns in this view
	fn stmt_select() -> sea_query::SelectStatement {
		Query::select()
			.from(CowIden(Self::Table::name()))
			.columns(Self::columns())
			.to_owned()
	}

	/// Create an update statement for this table
	/// with all columns and values in this view
	fn stmt_update(self) -> Result<sea_query::UpdateStatement> {
		let mut query = Query::update();
		query
			.table(CowIden(Self::Table::name()))
			.values(
				Self::columns()
					.into_iter()
					.zip(self.into_values().0.into_iter().map(|v| v.into())),
			)
			.to_owned()
			.xok()
	}

	async fn insert(self, conn: &impl Connection) -> Result<()> {
		conn.execute(self.stmt_insert()?).await
	}
	async fn insert_uncached(self, conn: &impl Connection) -> Result<()> {
		conn.execute_uncached(self.stmt_insert()?).await
	}


	async fn update_self(self, conn: &impl Connection) -> Result<()> {
		let kvp = self.primary_kvp()?;
		let mut stmt = self.stmt_update()?;
		stmt.and_where(Expr::col(kvp.0).eq(kvp.1));
		// println!("{:?}", stmt.build(sea_query::SqliteQueryBuilder));
		conn.execute(stmt).await
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sea_query::SqliteQueryBuilder;
	use sea_query::ValueType;
	use sweet::prelude::*;

	#[derive(Default, Table)]
	struct MyTable {
		id: u32,
		name: String,
	}

	#[test]
	fn value() {
		let val = sea_query::Value::from(3u32);
		// ValueType::type_name(val.clone()).unwrap();
		// let val: u32 = sea_query::ValueType::try_from(val.clone()).unwrap();

		// let val: u32 = u32::try_from.try_from().unwrap();
	}

	#[test]
	fn update() {
		let stmt = MyTable::default()
			.stmt_update()
			.unwrap()
			.build(SqliteQueryBuilder)
			.0;
		expect(stmt).to_be(
			"UPDATE \"my_table\" SET \"id\" = ?, \"name\" = ?".to_string(),
		);
	}
}
