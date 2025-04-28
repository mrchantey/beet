use crate::prelude::*;
use anyhow::Result;
use sea_query::QueryStatementBuilder;
use sea_query::SchemaBuilder;
use sea_query::SchemaStatementBuilder;
use sea_query::Values;

pub trait Statement<M>: Sized {
	fn build<T: SchemaBuilder>(&self, schema_builder: T) -> (String, Values);
	async fn execute(self, conn: &impl Connection) -> Result<()> {
		conn.execute(self).await
	}
	async fn query(self, conn: &impl Connection) -> Result<SeaQueryRows> {
		conn.query(self).await
	}
}

pub struct SchemaStatementBuilderMarker;

impl<T: SchemaStatementBuilder> Statement<SchemaStatementBuilderMarker> for T {
	fn build<U: SchemaBuilder>(&self, schema_builder: U) -> (String, Values) {
		(T::build(self, schema_builder), Values(Vec::new()))
	}
}

pub struct QueryStatementBuilderMarker;

impl<T: QueryStatementBuilder> Statement<QueryStatementBuilderMarker> for T {
	fn build<U: SchemaBuilder>(&self, schema_builder: U) -> (String, Values) {
		T::build_any(self, &schema_builder)
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
		expect(values.0).to_be(vec![3u32.into()]);
	}
}
