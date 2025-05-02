use crate::prelude::*;
use anyhow::Result;
use std::pin::Pin;
use sweet::prelude::*;



impl CachedStatementInner for limbo::Statement {
	fn box_clone(&self) -> Box<dyn CachedStatementInner> {
		Box::new(self.clone())
	}

	fn reset(&mut self) {
		// limbo::Statement::reset(self);
		unimplemented!("reset not implemented for limbo::Statement");
	}

	#[allow(unused)]
	fn execute<'a>(
		&'a mut self,
		row: Row,
	) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
		Box::pin(async move {
			limbo::Statement::execute(self, row.into_other::<limbo::Value>()?)
				.await?
				.xmap(LimboUtils::step_result_err)?
				.xok()
		})
	}
	fn query<'a>(
		&'a mut self,
		row: Row,
	) -> Pin<Box<dyn Future<Output = Result<Rows>> + 'a>> {
		Box::pin(async move {
			limbo::Statement::query(self, row.into_other::<limbo::Value>()?)
				.await?
				.xmap(LimboUtils::collect_rows)
				.await?
				.xok()
		})
	}
}
