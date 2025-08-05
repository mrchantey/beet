use crate::prelude::*;
use anyhow::Result;
use rapidhash::RapidHashMap;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(not(all(feature = "tokio", not(target_arch = "wasm32"))))]
type RwLock<T> = std::sync::RwLock<T>;

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
type RwLock<T> = tokio::sync::RwLock<T>;


#[derive(Default, Clone)]
pub struct CachedStatementMap(
	pub(crate) Arc<RwLock<RapidHashMap<u64, CachedStatement>>>,
);

impl std::ops::Deref for CachedStatementMap {
	type Target = RwLock<RapidHashMap<u64, CachedStatement>>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl CachedStatementMap {
	/// If a statement is already cached, runs the statement,
	/// otherwise prepares the statement, runs and then caches it
	///
	/// In native environments with tokio feature a [`tokio::sync::RwLock`] is used
	pub async fn get_or_prepare<'a>(
		&self,
		stmt: &'a str,
		prepare: impl FnOnce() -> Pin<
			Box<dyn 'a + Future<Output = Result<CachedStatement>>>,
		>,
	) -> Result<CachedStatement> {
		let key = rapidhash::rapidhash(stmt.as_bytes());
		#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
		if let Some(cached) = self.read().await.get(&key) {
			Ok(cached.clone())
		} else {
			let cached = prepare().await?;
			self.write().await.insert(key, cached.clone());
			Ok(cached)
		}
		#[cfg(not(all(feature = "tokio", not(target_arch = "wasm32"))))]
		if let Some(cached) = self.read().expect("poisoned").get(&key) {
			Ok(cached.clone())
		} else {
			let cached = prepare().await?;
			self.write().expect("poisoned").insert(key, cached.clone());
			Ok(cached)
		}
	}
}

/// A statement that has been compiled
pub struct CachedStatement {
	pub inner: Box<dyn CachedStatementInner>,
}
impl Clone for CachedStatement {
	fn clone(&self) -> Self {
		CachedStatement {
			inner: self.inner.box_clone(),
		}
	}
}

impl CachedStatement {
	pub fn new(inner: Box<dyn CachedStatementInner>) -> Self { Self { inner } }
	pub async fn execute<'a>(&'a mut self, row: Row) -> Result<()> {
		self.inner.reset();
		self.inner.execute(row).await
	}
	pub async fn query<'a>(&'a mut self, row: Row) -> Result<Rows> {
		self.inner.reset();
		self.inner.query(row).await
	}
}

/// All the annoying clone and pin stuff wrapping execute and query calls
pub trait CachedStatementInner: 'static + Send + Sync {
	fn box_clone(&self) -> Box<dyn CachedStatementInner>;

	/// Called before executing or querying
	fn reset(&mut self);

	fn execute<'a>(
		&'a mut self,
		row: Row,
	) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>>;

	fn query<'a>(
		&'a mut self,
		row: Row,
	) -> Pin<Box<dyn Future<Output = Result<Rows>> + 'a>>;
}




#[cfg(test)]
#[cfg(all(feature = "libsql", not(target_arch = "wasm32")))]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Clone, Table)]
	struct MyTable {
		name: String,
	}


	#[sweet::test]
	async fn works() {
		use sea_query::SqliteQueryBuilder;

		let db = Database::new().await.unwrap();
		db.create_table::<MyTable>().await.unwrap();
		let row = MyTable { name: "foo".into() };
		let stmt = row.clone().stmt_insert().unwrap();
		db.insert(row).await.unwrap();
		expect(db.statement_cache.read().await.len()).to_be(1);
		expect(
			db.statement_cache
				.read()
				.await
				.get(&rapidhash::rapidhash(
					stmt.build_any(&SqliteQueryBuilder).0.as_bytes(),
				))
				.is_some(),
		)
		.to_be_true();
	}
}
