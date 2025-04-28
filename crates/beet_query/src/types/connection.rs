use crate::prelude::*;
use anyhow::Result;



/// Unified connection type for all supported backends.
/// Calling `Connection::new()` will create a new connection
/// to the database, selecting backends in the following order based
/// on the features enabled:
/// - `libsql`
/// - `limbo`
#[derive(Clone)]
pub enum Connection {
	#[cfg(feature = "libsql")]
	Libsql(libsql::Connection),
	#[cfg(feature = "limbo")]
	Limbo(limbo::Connection),
}

impl Connection {}

impl ConnectionInner for Connection {
	fn statement_builder(&self) -> Box<dyn StatementBuilder> {
		match self {
			#[cfg(feature = "libsql")]
			Self::Libsql(conn) => conn.statement_builder(),
			#[cfg(feature = "limbo")]
			Self::Limbo(conn) => conn.statement_builder(),
		}
	}

	async fn new() -> Result<Self> {
		#[cfg(feature = "libsql")]
		return Ok(Self::Libsql(libsql::Connection::new().await?));
		#[cfg(feature = "limbo")]
		#[allow(unused)]
		return Ok(Self::Limbo(limbo::Connection::new().await?));
	}
	async fn execute_uncached<M>(
		&self,
		stmt: &impl Statement<M>,
	) -> Result<()> {
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
}

pub trait ConnectionInner: Sized {
	/// The sql builder flavor of this connection,
	/// usually one of [`SqliteQueryBuilder`], [`PostgreSqlQueryBuilder`], [`MySqlQueryBuilder`]
	fn statement_builder(&self) -> Box<dyn StatementBuilder>;
	/// Create a new connection, each implementation may
	/// look for environment variables to decide how to connect,
	/// or whether to fall back to an in-memory database.
	async fn new() -> Result<Self>;

	/// Execute a statement without caching it. This is usually used
	/// by [`Schema`](sea_query::SchemaStatementBuilder) statements like
	/// creating tables or indexes.
	async fn execute_uncached<M>(&self, stmt: &impl Statement<M>)
	-> Result<()>;
	/// Prepare a statement for execution.
	async fn prepare(&self, sql: &str) -> Result<CachedStatement>;
}
