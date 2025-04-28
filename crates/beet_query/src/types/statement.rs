use crate::prelude::*;
use anyhow::Result;
use sea_query::QueryBuilder;
use sea_query::QueryStatementBuilder;
use sea_query::SchemaBuilder;
use sea_query::SchemaStatementBuilder;
use sweet::prelude::*;

pub trait StatementBuilder:SchemaBuilder+ QueryBuilder{}
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

pub trait Statement<M>: Sized {
	/// Define the type of statement this is, which can be used
	/// to determine a caching strategy.
	fn statement_type(&self) -> StatementType;

	/// Build a [`SeaQuery`] statement into [`beet::Rows`](Rows)
	fn build(
		&self,
		schema_builder: &dyn StatementBuilder,
	) -> ConvertValueResult<(String, Row)>;
	async fn execute(self, conn: &Connection) -> Result<()> {
		conn.execute(self).await
	}
	async fn query(self, conn: &Connection) -> Result<Rows> {
		conn.query(self).await
	}
}

pub struct SchemaStatementBuilderMarker;

impl<T: SchemaStatementBuilder> Statement<SchemaStatementBuilderMarker> for T {
	fn statement_type(&self) -> StatementType { StatementType::Schema }

	fn build(
		&self,
		schema_builder: &dyn StatementBuilder,
	) -> ConvertValueResult<(String, Row)> {
		(T::build_any(self, schema_builder), Row::default()).xok()
	}
}

pub struct QueryStatementBuilderMarker;

impl<T: QueryStatementBuilder> Statement<QueryStatementBuilderMarker> for T {
	fn statement_type(&self) -> StatementType { StatementType::Query }

	fn build(
		&self,
		schema_builder: &dyn StatementBuilder,
	) -> ConvertValueResult<(String, Row)> {
		T::build_any(self, schema_builder)
			.xmap(|(sql, values)| (sql, values.into_row()?).xok())
	}
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
		expect(placeholder).to_be("INSERT INTO \"foo\" (\"bar\") VALUES (?)");
		expect(values.0).to_be(vec![3i64.into()]);
	}
}
