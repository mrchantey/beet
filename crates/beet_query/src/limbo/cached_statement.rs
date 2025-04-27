use crate::prelude::*;
use anyhow::Result;
use sea_query::Values;
use std::pin::Pin;
use sweet::prelude::*;



impl CachedStatementInner for limbo::Statement {
	fn box_clone(&self) -> Box<dyn CachedStatementInner> {
		Box::new(self.clone())
	}
	fn execute<'a>(
		&'a mut self,
		values: Values,
	) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
		Box::pin(async move {
			let step_result =
				limbo::Statement::execute(self, values.into_limbo_values())
					.await?;
			step_result_err(step_result)?;
			Ok(())
		})
	}
	fn query<'a>(
		&'a mut self,
		values: Values,
	) -> Pin<Box<dyn Future<Output = Result<Rows>> + 'a>> {
		Box::pin(async move {
			limbo::Statement::query(self, values.into_limbo_values())
				.await?
				.collect()?
				.into_iter()
				.map(|row| {
					row.into_inner()
						.into_iter()
						.map(|v| {
							v.to_owned()
								// we nee an intermediaray type because
								// limbo_core::OwnedValue != limbo::Value
								.xinto::<limbo::Value>()
								.into_sea_query_value()
						})
						.collect::<Vec<_>>()
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}
}

