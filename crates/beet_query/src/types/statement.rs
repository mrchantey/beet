use crate::prelude::*;
use anyhow::Result;
use sea_query::QueryStatementBuilder;
use sea_query::SchemaBuilder;
use sea_query::SchemaStatementBuilder;
use sweet::prelude::*;

pub trait Statement<M>: Sized {
	/// Build a [`SeaQuery`] statement into [`beet::Rows`](Rows)
	fn build<T: SchemaBuilder>(
		&self,
		schema_builder: T,
	) -> ConvertValueResult<(String, Row)>;
	async fn execute(self, conn: &impl ConnectionInner) -> Result<()> {
		conn.execute(self).await
	}
	async fn query(self, conn: &impl ConnectionInner) -> Result<Rows> {
		conn.query(self).await
	}
}

pub struct SchemaStatementBuilderMarker;

impl<T: SchemaStatementBuilder> Statement<SchemaStatementBuilderMarker> for T {
	fn build<U: SchemaBuilder>(
		&self,
		schema_builder: U,
	) -> ConvertValueResult<(String, Row)> {
		(T::build(self, schema_builder), Row::default()).xok()
	}
}

pub struct QueryStatementBuilderMarker;

impl<T: QueryStatementBuilder> Statement<QueryStatementBuilderMarker> for T {
	fn build<U: SchemaBuilder>(
		&self,
		schema_builder: U,
	) -> ConvertValueResult<(String, Row)> {
		T::build_any(self, &schema_builder)
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
