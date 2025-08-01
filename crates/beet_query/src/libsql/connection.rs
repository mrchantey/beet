use crate::libsql::cached_statement::LibsqlStatementWrapper;
use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::*;
use sea_query::SqliteQueryBuilder;

#[cfg(feature = "libsql")]
impl ConnectionInner for libsql::Connection {
	fn statement_builder(&self) -> Box<dyn StatementBuilder> {
		Box::new(SqliteQueryBuilder)
	}

	async fn new() -> Result<libsql::Connection> {
		if let Ok(url) = std::env::var("LIBSQL_URL") {
			let token = std::env::var("LIBSQL_AUTH_TOKEN")?;
			libsql::Builder::new_remote(url, token)
				.build()
				.await?
				.connect()?
				.xok()
		} else {
			libsql::Builder::new_local(":memory:")
				.build()
				.await?
				.connect()?
				.xok()
		}
	}

	async fn execute_uncached(&self, stmt: &impl Statement) -> Result<()> {
		let (sql, row) = stmt.build(&*self.statement_builder())?;
		self.execute(&sql, row.into_other::<libsql::Value>()?)
			.await?;
		Ok(())
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		let stmt = self.prepare(sql).await?;
		CachedStatement::new(Box::new(LibsqlStatementWrapper::new(stmt))).xok()
	}
}
