use sea_query::QueryStatementBuilder;
use sea_query::SchemaBuilder;
use sea_query::SchemaStatementBuilder;
use sea_query::Values;





pub trait Statement<M> {
	fn build<T: SchemaBuilder>(&self, schema_builder: T) -> (String, Values);
}

pub struct SchemaStatementBuilderMarker;

impl<T: SchemaStatementBuilder> Statement<SchemaStatementBuilderMarker> for T {
	fn build<U: SchemaBuilder>(&self, schema_builder: U) -> (String, Values) {
		(self.build(schema_builder), Values(Vec::new()))
	}
}

pub struct QueryStatementBuilderMarker;

impl<T: QueryStatementBuilder> Statement<QueryStatementBuilderMarker> for T {
	fn build<U: SchemaBuilder>(&self, schema_builder: U) -> (String, Values) {
		self.build_any(&schema_builder)
	}
}


pub struct CachedStatement {}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sea_query::SqliteQueryBuilder;

	#[derive(Table)]
	struct Foo {
		bar: u32,
	}


	#[test]
	fn works() {
		let stmt2 = Foo { bar: 3 }.stmt_insert().unwrap();
		let (placeholder2, values2) = stmt2.build_any(&SqliteQueryBuilder);
		println!("SQL: {}", placeholder2);
		println!("Values: {:?}", values2);
	}
}
