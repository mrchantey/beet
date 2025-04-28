use crate::prelude::*;
use anyhow::Result;


#[derive(Clone)]
pub struct Database {
	pub connection: Connection,
	pub statement_cache: CachedStatementMap,
}



impl Database {
	pub async fn new() -> Result<Self> {
		let connection = Connection::new().await?;
		let statement_cache = Default::default();
		Ok(Self {
			connection,
			statement_cache,
		})
	}

	async fn get_or_prepare(&self, sql: &str) -> Result<CachedStatement> {
		self.statement_cache
			.get_or_prepare(sql, || Box::pin(self.connection.prepare(sql)))
			.await
	}

	/// Execute a statement:
	///
	/// ## Caching
	/// The caching strategy is automatically determined based on
	/// the statement type:
	/// - [`Schema`](sea_query::SchemaStatementBuilder) statements like
	///  creating tables or indexes are executed without caching
	/// - [`Query`](sea_query::QueryStatementBuilder) statements like
	/// `SELECT`, `INSERT`, `UPDATE` and `DELETE are cached
	pub async fn execute<M>(&self, stmt: &impl Statement<M>) -> Result<()> {
		if stmt.statement_type() == StatementType::Schema {
			self.connection.execute_uncached(stmt).await
		} else {
			self.execute_cached(stmt).await
		}
	}
	pub async fn query<M>(&self, stmt: &impl Statement<M>) -> Result<Rows> {
		let (sql, values) =
			stmt.build(&*self.connection.statement_builder())?;
		self.get_or_prepare(&sql).await?.query(values).await
	}

	/// Execute a statement and automatically cache it for next call. This
	/// is usually used by [`Query`](sea_query::QueryStatementBuilder) statements
	/// like `SELECT`, `INSERT`, `UPDATE` and `DELETE`.
	async fn execute_cached<M>(&self, stmt: &impl Statement<M>) -> Result<()> {
		let (sql, values) =
			stmt.build(&*self.connection.statement_builder())?;
		self.get_or_prepare(&sql).await?.execute(values).await
	}



	/// Execute a `CREATE TABLE` statement with this table's [`stmt_create_table`](Table::stmt_create_table)
	pub async fn create_table<T: Table>(&self) -> Result<()> {
		self.execute(&T::stmt_create_table()).await
	}

	pub async fn insert<T: TableView>(&self, value: T) -> Result<()> {
		self.execute(&value.stmt_insert()?).await
	}
}
