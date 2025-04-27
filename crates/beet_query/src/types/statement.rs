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
