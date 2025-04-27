use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;


pub trait Connection {
	async fn execute<M>(&self, stmt: impl Statement<M>) -> Result<()>;
}


#[cfg(feature = "limbo")]
impl Connection for limbo::Connection {
	async fn execute<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, values) = stmt.build(SqliteQueryBuilder);
		self.execute(&sql, values.into_limbo_values()).await?;
		Ok(())
	}
}
