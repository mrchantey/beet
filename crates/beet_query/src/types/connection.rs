use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;

pub trait Connection {
	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()>;
	async fn execute<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, values) = stmt.build(SqliteQueryBuilder);
		CachedStatement::get_or_prepare(&sql, || {
			Box::pin(Connection::prepare(self, &sql))
		})
		.await?
		.execute(values)
		.await?;
		Ok(())
	}

	async fn query<M>(&self, stmt: impl Statement<M>) -> Result<SeaQueryRows> {
		let (sql, values) = stmt.build(SqliteQueryBuilder);
		CachedStatement::get_or_prepare(&sql, || {
			Box::pin(Connection::prepare(self, &sql))
		})
		.await?
		.query(values)
		.await
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement>;
}
