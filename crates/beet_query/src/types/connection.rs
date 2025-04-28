use crate::prelude::*;
use anyhow::Result;
use sea_query::SqliteQueryBuilder;

/// Unified connection type for all supported backends.
/// Calling `Connection::new()` will create a new connection
/// to the database, selecting backends in the following order based
/// on the features enabled:
/// - `libsql`
/// - `limbo`
pub enum Connection {
	#[cfg(feature = "libsql")]
	Libsql(libsql::Connection),
	#[cfg(feature = "limbo")]
	Limbo(limbo::Connection),
}

impl ConnectionInner for Connection {
	async fn new() -> Result<Self> {
		#[cfg(feature = "libsql")]
		return Ok(Self::Libsql(libsql::Connection::new().await?));
		#[cfg(feature = "limbo")]
		#[allow(unused)]
		return Ok(Self::Limbo(limbo::Connection::new().await?));
	}
	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		match self {
			#[cfg(feature = "libsql")]
			Self::Libsql(conn) => ConnectionInner::execute_uncached(conn, stmt).await,
			#[cfg(feature = "limbo")]
			Self::Limbo(conn) => ConnectionInner::execute_uncached(conn, stmt).await,
		}
	}

	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		match self {
			#[cfg(feature = "libsql")]
			Self::Libsql(conn) => ConnectionInner::prepare(conn, sql).await,
			#[cfg(feature = "limbo")]
			Self::Limbo(conn) => ConnectionInner::prepare(conn, sql).await,
		}
	}
	async fn execute<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		match self {
			#[cfg(feature = "libsql")]
			Self::Libsql(conn) => ConnectionInner::execute(conn, stmt).await,
			#[cfg(feature = "limbo")]
			Self::Limbo(conn) => ConnectionInner::execute(conn, stmt).await,
		}
	}
}

pub trait ConnectionInner: Sized {
	/// Create a new connection, each implementation may
	/// look for environment variables to decide how to connect,
	/// or whether to fall back to an in-memory database.
	async fn new() -> Result<Self>;

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
