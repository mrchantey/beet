use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;
use sweet::prelude::PipelineTarget;

#[cfg(feature = "limbo")]
impl Connection for limbo::Connection {
	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, values) = stmt.build(SqliteQueryBuilder);
		self.execute(&sql, values.into_limbo_values()).await?;
		Ok(())
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		CachedStatement::new(Box::new(self.prepare(sql).await?)).xok()
	}
}
