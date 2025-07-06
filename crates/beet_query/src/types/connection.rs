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
			#[cfg(not(any(feature = "libsql", feature = "limbo")))]
			_ => panic!("please enable a database feature"),
		}
	}

	async fn new() -> Result<Self> {
		#[cfg(feature = "libsql")]
		return Ok(Self::Libsql(libsql::Connection::new().await?));
		#[cfg(feature = "limbo")]
		#[allow(unused)]
		return Ok(Self::Limbo(limbo::Connection::new().await?));
		#[cfg(not(any(feature = "libsql", feature = "limbo")))]
		panic!("please enable a database feature");
	}
	#[allow(unused)]
	async fn execute_uncached(&self, stmt: &impl Statement) -> Result<()> {
		match self {
			#[cfg(feature = "libsql")]
			Self::Libsql(conn) => ConnectionInner::execute_uncached(conn, stmt).await,
			#[cfg(feature = "limbo")]
			Self::Limbo(conn) => ConnectionInner::execute_uncached(conn, stmt).await,
			#[cfg(not(any(feature = "libsql", feature = "limbo")))]
			_ => panic!("please enable a database feature"),
		}
	}

	#[allow(unused)]
	async fn prepare(&self, sql: &str) -> Result<CachedStatement> {
		match self {
			#[cfg(feature = "libsql")]
			Self::Libsql(conn) => ConnectionInner::prepare(conn, sql).await,
			#[cfg(feature = "limbo")]
			Self::Limbo(conn) => ConnectionInner::prepare(conn, sql).await,
			#[cfg(not(any(feature = "libsql", feature = "limbo")))]
			_ => panic!("please enable a database feature"),
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
	async fn execute_uncached(&self, stmt: &impl Statement) -> Result<()>;
	/// Prepare a statement for execution.
	async fn prepare(&self, sql: &str) -> Result<CachedStatement>;
}
