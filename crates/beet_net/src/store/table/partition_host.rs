use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

/// Many partitions served by name in one process, each its own SQLite
/// database and each one's requests answered one at a time, the way a
/// Durable Object answers its own: the in-process host of the same
/// [`TableWire`] protocol an object speaks, so a runtime moves between a
/// native host and Cloudflare by declaration alone. The precedent is Orleans'
/// virtual actors, an activation per name wherever it is hosted.
///
/// The declaration's own partition is `root.db` in the host's directory, and
/// partition `id` is that database's partition file
/// ([`SqliteStore::partition_store`]), so every partition is one exportable
/// file. [`memory`](Self::memory) holds them in memory instead, for a test.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let host = PartitionHost::memory("my-host");
/// let store = TableStore::new(host.client().await?);
/// let lobby = store.partition("lobby-42")?;
/// lobby.table::<TableItem<u32>>().push(TableItem::new(7)).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct PartitionHost {
	/// The declaration's own partition, whose partition files the others
	/// are.
	root: SqliteStore,
	/// One lock per activated partition, `None` the declaration's own.
	locks: Arc<Mutex<HashMap<Option<PartitionId>, Arc<async_lock::Mutex<()>>>>>,
}

impl PartitionHost {
	/// A host of SQLite files under `dir`.
	pub fn new(dir: impl Into<AbsPath>) -> Self {
		Self::over(SqliteStore::new(dir.into().join("root.db")))
	}

	/// A host holding its partitions in memory under `name`.
	pub fn memory(name: impl AsRef<str>) -> Self {
		Self::over(SqliteStore::memory(format!("{}/root.db", name.as_ref())))
	}

	fn over(root: SqliteStore) -> Self {
		Self {
			root,
			locks: default(),
		}
	}

	/// The database of partition `id`, the declaration's own for `None`.
	pub fn database(&self, id: Option<&PartitionId>) -> SqliteStore {
		match id {
			Some(id) => self.root.partition_store(id),
			None => self.root.clone(),
		}
	}

	/// Answer `request` from the partition its path names, after every
	/// earlier request to that partition is answered.
	pub async fn serve(&self, request: Request) -> Response {
		let (id, path) =
			match TableWire::split_partition(request.request_parts().path()) {
				Ok(split) => split,
				Err(err) => {
					return Response::status_text(
						StatusCode::BAD_REQUEST,
						err.to_string(),
					);
				}
			};
		let lock = self
			.locks
			.lock()
			.unwrap_or_else(|err| err.into_inner())
			.entry(id.clone())
			.or_default()
			.clone();
		let _guard = lock.lock().await;
		TableWire::answer(&self.database(id.as_ref()), &path, request).await
	}

	/// A client speaking to this host in process, its capabilities fetched.
	pub async fn client(&self) -> Result<TableWireClient> {
		let host = self.clone();
		TableWireClient::connect(TableWireTransport::new(
			format!("partition-host:{}", self.root.path()),
			move |request| {
				let host = host.clone();
				Box::pin(async move { host.serve(request).await.xok() })
					as SendBoxedFuture<Result<Response>>
			},
		))
		.await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The suite through the protocol, the host answering in process.
	#[beet_core::test(timeout_ms = 60_000)]
	async fn suite() {
		let host = PartitionHost::memory("suite-host");
		table_test::run(TableStore::new(host.client().await.unwrap())).await;
	}

	/// Each partition is its own file beside the declaration's.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn partitions_are_files() {
		let dir = TempDir::new_ws().unwrap();
		let host = PartitionHost::new(dir.path().clone());
		let store = TableStore::new(host.client().await.unwrap());
		store
			.partition("lobby-42")
			.unwrap()
			.table::<table_test::NamedRow>()
			.put(table_test::NamedRow::new("x", 1))
			.await
			.unwrap();
		fs_ext::exists(dir.path().join("root/lobby-42.db"))
			.unwrap()
			.xpect_true();
	}

	/// A partition's requests are answered one at a time: concurrent
	/// unconditional increments read and write without interleaving only
	/// because each conditional write retries, and the host never answers
	/// two at once.
	#[beet_core::test(timeout_ms = 60_000)]
	async fn serializes_a_partition() {
		let host = PartitionHost::memory("serial-host");
		let store = TableStore::new(host.client().await.unwrap());
		let rows = store
			.partition("p")
			.unwrap()
			.table::<table_test::NamedRow>();
		rows.put(table_test::NamedRow::new("n", 0)).await.unwrap();
		(0..4)
			.map(|_| {
				let rows = rows.clone();
				async move {
					loop {
						let (mut row, cid) =
							rows.get_with_cid("n").await?.unwrap();
						row.value += 1;
						match rows.put_if(row, Precondition::Matches(cid)).await
						{
							Ok(_) => return Ok::<_, BevyError>(()),
							Err(err) if TableError::is_conflict(&err) => {}
							Err(err) => return Err(err),
						}
					}
				}
			})
			.xmap(async_ext::try_join_all)
			.await
			.unwrap();
		rows.get("n").await.unwrap().unwrap().value.xpect_eq(4);
	}
}
