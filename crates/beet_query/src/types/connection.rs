use crate::prelude::*;
use anyhow::Result;

#[derive(Clone)]
pub struct Connection {
	pub variant: ConnectionVariant,
	pub cached_statements: CachedStatementMap,
}



impl Connection {
	pub async fn new() -> Result<Self> {
		let variant = ConnectionVariant::new().await?;
		let cached_statements = Default::default();
		Ok(Self {
			variant,
			cached_statements,
		})
	}

	async fn get_or_prepare(&self, sql: &str) -> Result<CachedStatement> {
		self.cached_statements
			.get_or_prepare(sql, || Box::pin(self.variant.prepare(sql)))
			.await
	}

	/// Execute a statement:
	///
	/// ## Caching
	/// The caching strategy is automatically determined based on
	/// the statement type:
	/// - [`Schema`](sea_query::SchemaStatementBuilder) statements like
	///  creating tables or indexes are executed without caching
	/// - [`Query`](sea_query::QueryStatementBuilder) statements like
	/// `SELECT`, `INSERT`, `UPDATE` and `DELETE are cached
	pub async fn execute<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		if stmt.statement_type() == StatementType::Schema {
			self.variant.execute_uncached(stmt).await
		} else {
			self.execute_cached(stmt).await
		}
	}
	pub async fn query<M>(&self, stmt: impl Statement<M>) -> Result<Rows> {
		let (sql, values) = stmt.build(&*self.variant.statement_builder())?;
		self.get_or_prepare(&sql).await?.query(values).await
	}

	/// Execute a statement and automatically cache it for next call. This
	/// is usually used by [`Query`](sea_query::QueryStatementBuilder) statements
	/// like `SELECT`, `INSERT`, `UPDATE` and `DELETE`.
	async fn execute_cached<M>(&self, stmt: impl Statement<M>) -> Result<()> {
		let (sql, values) = stmt.build(&*self.variant.statement_builder())?;
		self.get_or_prepare(&sql).await?.execute(values).await
	}
}


/// Unified connection type for all supported backends.
/// Calling `Connection::new()` will create a new connection
/// to the database, selecting backends in the following order based
/// on the features enabled:
/// - `libsql`
/// - `limbo`
#[derive(Clone)]
pub enum ConnectionVariant {
	#[cfg(feature = "libsql")]
	Libsql(libsql::Connection),
	#[cfg(feature = "limbo")]
	Limbo(limbo::Connection),
}

impl ConnectionVariant {}

impl ConnectionInner for ConnectionVariant {
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
	async fn execute_uncached<M>(&self, stmt: impl Statement<M>) -> Result<()>;
	/// Prepare a statement for execution.
	async fn prepare(&self, sql: &str) -> Result<CachedStatement>;
}
