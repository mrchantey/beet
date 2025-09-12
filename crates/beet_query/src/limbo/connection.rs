use crate::prelude::*;
use anyhow::Result;
use beet_core::prelude::*;
use sea_query::SqliteQueryBuilder;


#[cfg(feature = "limbo")]
impl ConnectionInner for limbo::Connection {
	fn statement_builder(&self) -> Box<dyn StatementBuilder> {
		Box::new(SqliteQueryBuilder)
	}

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

	async fn execute_uncached(&self, stmt: &impl Statement) -> Result<()> {
		let (sql, row) = stmt.build(&*self.statement_builder())?;
		self.execute(&sql, row.into_other::<limbo::Value>()?)
			.await?;
		Ok(())
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		CachedStatement::new(Box::new(self.prepare(sql).await?)).xok()
	}
}
