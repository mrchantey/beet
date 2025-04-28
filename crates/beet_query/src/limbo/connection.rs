use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;
use sweet::prelude::PipelineTarget;


#[cfg(feature = "limbo")]
impl ConnectionInner for limbo::Connection {
	async fn new() -> Result<Self> {
		if let Ok(_url) = std::env::var("LIMBO_URL") {
			unimplemented!()
			// let token = std::env::var("LIMBO_AUTH_TOKEN")?;
			// limbo::Builder::new_remote(url, token)
			// 	.build()
			// 	.await?
			// 	.connect()?
			// 	.xok()
		} else {
			limbo::Builder::new_local(":memory:")
				.build()
				.await?
				.connect()?
				.xok()
		}
	}

	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, row) = stmt.build(SqliteQueryBuilder)?;
		self.execute(&sql, row.into_other::<limbo::Value>()?)
			.await?;
		Ok(())
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		CachedStatement::new(Box::new(self.prepare(sql).await?)).xok()
	}
}
