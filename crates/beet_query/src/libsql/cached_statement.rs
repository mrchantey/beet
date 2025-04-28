use crate::prelude::*;
use anyhow::Result;
use sea_query::Values;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use sweet::prelude::*;

/// A wrapper around libsql::Statement that can be cloned
#[derive(Clone)]
pub struct LibsqlStatementWrapper {
	statement: Arc<Mutex<libsql::Statement>>,
}

impl LibsqlStatementWrapper {
	/// Create a new wrapper around a libsql::Statement
	pub fn new(statement: libsql::Statement) -> Self {
		Self {
			statement: Arc::new(Mutex::new(statement)),
		}
	}
}

impl CachedStatementInner for LibsqlStatementWrapper {
	fn box_clone(&self) -> Box<dyn CachedStatementInner> {
		Box::new(self.clone())
	}
	fn reset(&mut self) {
		libsql::Statement::reset(&mut *self.statement.lock().unwrap());
	}

	fn execute<'a>(
		&'a mut self,
		values: Values,
	) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
		Box::pin(async move {
			let mut stmt = self.statement.lock().unwrap();
			libsql::Statement::execute(&mut *stmt, values.into_libsql_values())
				.await?
				.xmap(LibsqlUtils::parse_step_result)
				.xmap(|_| {})
				.xok()
		})
	}

	fn query<'a>(
		&'a mut self,
		values: Values,
	) -> Pin<Box<dyn Future<Output = Result<SeaQueryRows>> + 'a>> {
		Box::pin(async move {
			let mut stmt = self.statement.lock().unwrap();
			libsql::Statement::query(&mut *stmt, values.into_libsql_values())
				.await?
				.xmap(LibsqlUtils::collect_rows)
				.await?
				.xok()
		})
	}
}
