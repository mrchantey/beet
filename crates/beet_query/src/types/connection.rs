use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;


pub struct Connection {
	// pub inner: Box<dyn ConnectionInner>,
}



pub trait ConnectionInner {
	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()>;

	async fn prepare(&self, sql: &str) -> Result<CachedStatement>;

	async fn execute<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, values) = stmt.build(SqliteQueryBuilder)?;
		CachedStatement::get_or_prepare(&sql, || {
			Box::pin(ConnectionInner::prepare(self, &sql))
		})
		.await?
		.execute(values)
		.await?;
		Ok(())
	}

	async fn query<M>(&self, stmt: impl Statement<M>) -> Result<Rows> {
		let (sql, values) = stmt.build(SqliteQueryBuilder)?;
		CachedStatement::get_or_prepare(&sql, || {
			Box::pin(ConnectionInner::prepare(self, &sql))
		})
		.await?
		.query(values)
		.await
	}
}
