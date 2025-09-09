use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::*;
use sea_query::DeleteStatement;
use sea_query::ForeignKeyCreateStatement;
use sea_query::ForeignKeyDropStatement;
use sea_query::IndexCreateStatement;
use sea_query::IndexDropStatement;
use sea_query::InsertStatement;
use sea_query::QueryBuilder;
use sea_query::QueryStatementBuilder;
use sea_query::SchemaBuilder;
use sea_query::SelectStatement;
use sea_query::TableAlterStatement;
use sea_query::TableCreateStatement;
use sea_query::TableDropStatement;
use sea_query::TableRenameStatement;
use sea_query::TableTruncateStatement;
use sea_query::UpdateStatement;
use sea_query::WithQuery;

pub trait StatementBuilder: SchemaBuilder + QueryBuilder {}
impl<T: SchemaBuilder + QueryBuilder> StatementBuilder for T {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StatementType {
	/// A statement that is used to create or modify the database schema
	/// e.g. create table, alter table, create index, create foreign key.
	Schema,
	/// A statement that is used to query the database,
	/// e.g. select, insert, update, delete.
	Query,
}

pub trait Statement: Sized {
	/// Define the type of statement this is, which can be used
	/// to determine a caching strategy.
	fn statement_type(&self) -> StatementType;

	/// Build a [`SeaQuery`] statement into [`beet::Rows`](Rows)
	fn build(
		&self,
		schema_builder: &dyn StatementBuilder,
	) -> ConvertValueResult<(String, Row)>;
	async fn execute(&self, db: &Database) -> Result<()> {
		db.execute(self).await
	}
	async fn query(&self, db: &Database) -> Result<Rows> {
		db.query(self).await
	}
}

pub struct SchemaStatementBuilderMarker;

macro_rules! impl_schema_statement {
	($($type:ty),* $(,)?) => {
		$(
			impl Statement for $type {
				fn statement_type(&self) -> StatementType { StatementType::Schema }

				fn build(
					&self,
					schema_builder: &dyn StatementBuilder,
				) -> ConvertValueResult<(String, Row)> {
					(Self::build_any(self, schema_builder), Row::default()).xok()
				}
			}
		)*
	}
}

impl_schema_statement! {
	TableCreateStatement,
	TableAlterStatement,
	TableDropStatement,
	TableRenameStatement,
	TableTruncateStatement,
	IndexCreateStatement,
	IndexDropStatement,
	ForeignKeyCreateStatement,
	ForeignKeyDropStatement,
}

macro_rules! impl_query_statement {
	($($type:ty),* $(,)?) => {
		$(
			impl Statement for $type {
				fn statement_type(&self) -> StatementType { StatementType::Query }

				fn build(
					&self,
					schema_builder: &dyn StatementBuilder,
				) -> ConvertValueResult<(String, Row)> {
					Self::build_any(self, schema_builder)
					.xmap(|(sql, values)| (sql, values.into_row()?).xok())
				}
			}
		)*
	}
}

impl_query_statement! {
	SelectStatement,
	InsertStatement,
	UpdateStatement,
	DeleteStatement,
	WithQuery
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sea_query::SqliteQueryBuilder;
	use sweet::prelude::*;

	#[derive(Table)]
	struct Foo {
		bar: u32,
	}


	#[test]
	fn works() {
		let stmt = Foo { bar: 3 }.stmt_insert().unwrap();
		let (placeholder, values) = stmt.build_any(&SqliteQueryBuilder);
		placeholder.xpect_eq("INSERT INTO \"foo\" (\"bar\") VALUES (?)");
		values.0.xpect_eq(vec![3i64.into()]);
	}
}
