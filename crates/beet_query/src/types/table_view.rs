use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::*;
use sea_query::Expr;
use sea_query::InsertStatement;
use sea_query::Query;
use sea_query::SimpleExpr;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DeserializeError {
	#[error("Row Length Mismatch: expected: {expected}, received: {received}")]
	RowLengthMismatch { expected: usize, received: usize },
	#[error("Type Mismatch: expected: {expected}, received: {received}")]
	TypeMismatch { expected: String, received: String },
	#[error("{0}")]
	ConvertValueError(#[from] ConvertValueError),
}

/// A trait for a partial view of a table,
/// used for insert, update and query statements
/// TODO all [sea_query::QueryStatement]
pub trait TableView: Sized {
	type PrimaryKey;

	/// The table this view is for
	type Table: Table;
	/// All columns for this insert view. This includes Optional columns,
	/// which will be set to ? in the insert statement if `None`.
	/// This must be the same length and order as [`Self::into_values`](TableView::into_values)
	fn columns() -> Vec<<Self::Table as Table>::Columns>;
	/// Like [`serde::Serialize`], converts the view into a [`Values`]
	/// for an insert or update statement.
	/// This must be the same length and order as [`Self::columns`](TableView::columns)
	fn into_row(self) -> ConvertValueResult<Row>;

	/// Like [`serde::Deserialize`], converts a row of values into this view.
	fn from_row(row: Row) -> Result<Self, DeserializeError>;
	/// Returns the value of the primary key for this table, its type
	/// should match [`Self::Table::Columns::primary_key`](Columns::primary_key)
	fn primary_value(&self) -> ConvertValueResult<Option<Value>> { Ok(None) }

	/// Create a [`sea_query::SimpleExpr`] stating that the field with
	/// its name is equal to the value provided
	fn expr_primary_key_eq(key_val: Self::PrimaryKey) -> Result<SimpleExpr>
	where
		Self::PrimaryKey: ConvertValue,
	{
		let Some(key_name) = <Self::Table as Table>::Columns::primary_key()
		else {
			return Err(anyhow::anyhow!(
				"No primary key defined for table {}",
				Self::Table::name()
			)
			.into());
		};

		Expr::col(key_name)
			.eq(key_val
				.into_value()?
				.into_other::<sea_query::SimpleExpr>()?)
			.xok()
	}

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

		let value = self.primary_value()?.ok_or_else(|| {
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
			.values(self.into_row()?.into_other()?)?
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
		Query::update()
			.table(CowIden(Self::Table::name()))
			.values(
				Self::columns()
					.into_iter()
					.zip(self.into_row()?.into_other()?),
			)
			.to_owned()
			.xok()
	}

	/// Create a delete statement for this table
	fn stmt_delete() -> sea_query::DeleteStatement {
		Query::delete()
			.from_table(CowIden(Self::Table::name()))
			.to_owned()
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sea_query::SqliteQueryBuilder;
	use sweet::prelude::*;

	#[derive(Default, Table)]
	struct MyTable {
		id: u32,
		name: String,
	}

	#[test]
	fn value() {
		// let val = sea_query::Value::from(3u32);
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
		stmt.xpect_eq(
			"UPDATE \"my_table\" SET \"id\" = ?, \"name\" = ?".to_string(),
		);
	}
}
