use crate::libsql::cached_statement::LibsqlStatementWrapper;
use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;
use sweet::prelude::PipelineTarget;

#[cfg(feature = "libsql")]
impl ConnectionInner for libsql::Connection {
	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, row) = stmt.build(SqliteQueryBuilder)?;
		self.execute(&sql, row.into_other::<libsql::Value>()?)
			.await?;
		Ok(())
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		let stmt = self.prepare(sql).await?;
		CachedStatement::new(Box::new(LibsqlStatementWrapper::new(stmt))).xok()
	}
}
