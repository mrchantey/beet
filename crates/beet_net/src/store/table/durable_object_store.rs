use crate::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;

/// A table store whose every partition is a Cloudflare Durable Object: one
/// object per partition, named by `id_from_name(<partition>)` (`@` for the
/// declaration's own), owning that partition's SQLite storage and answering
/// the [`TableWire`] protocol over it. The Worker routes and does stateless
/// work; anything that reads then writes a partition runs in its object.
///
/// Resolved by ancestry like every store, its [`TableStore`] and
/// [`BlobStore`] (the declaration's own partition's blob side) landing on the
/// declaring entity:
/// - in a Worker (`wasm32` with `cloudflare`), a [`TableWireClient`] sending
///   each request to its partition's object through the namespace `binding`;
/// - natively (`sqlite`), the same client over an in-process
///   [`PartitionHost`] keeping each partition as a SQLite file under `dir`
///   (by default the workspace's `target/stores/<binding>`), its requests to
///   one partition answered one at a time as an object answers its own.
///
/// So a runtime moves between a native host and Cloudflare by where it runs,
/// and nothing that reads the store knows which. The Worker's deploy renders
/// the binding and its `new_sqlite_classes` migration for the one object
/// class beet exports, [`DurableObjectStore::CLASS`].
///
/// ```html
/// <DurableObjectStore bx:ref="tables" binding="TABLES"/>
/// ```
///
/// A Durable Object's row is at most 2MB, a row and a blob alike.
#[derive(Debug, Clone, Get, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = DurableObjectStore::on_insert)]
pub struct DurableObjectStore {
	/// The Durable Object namespace binding, ie `TABLES` in `env.TABLES`.
	binding: SmolStr,
	/// Where the native host keeps its databases, by default the
	/// workspace's `target/stores/<binding>`.
	#[reflect(default)]
	dir: Option<AbsPath>,
}

impl Default for DurableObjectStore {
	fn default() -> Self { Self::new("TABLES") }
}

impl DurableObjectStore {
	/// The object class every partition is an instance of, the one a Worker
	/// built from beet exports.
	pub const CLASS: &'static str = "BeetTableObject";
	/// The object name of the declaration's own partition: never a partition
	/// id, whose alphabet has no `@`.
	pub const ROOT: &'static str = "@";
	/// What every object supports, the same as a SQLite file.
	const CAPABILITIES: [Capability; 2] =
		[Capability::AtomicApply, Capability::Indexes];

	/// The store of the namespace `binding`.
	pub fn new(binding: impl Into<SmolStr>) -> Self {
		Self {
			binding: binding.into(),
			dir: None,
		}
	}

	/// Keep the native host's databases under `dir`.
	pub fn with_dir(mut self, dir: impl Into<AbsPath>) -> Self {
		self.dir = Some(dir.into());
		self
	}

	/// The client of this store where the process runs, see
	/// [`DurableObjectStore`]; in a build with neither backend, a client
	/// refusing every request.
	pub fn client(&self) -> Result<TableWireClient> {
		cfg_if! {
			if #[cfg(all(target_arch = "wasm32", feature = "cloudflare"))] {
				let binding = self.binding.clone();
				TableWireClient::new(TableWireTransport::new(
					format!("durable-object:{binding}"),
					move |request| {
						let binding = binding.clone();
						Box::pin(send_wrapper::SendWrapper::new(async move {
							DurableObjectStore::fetch(&binding, request).await
						})) as SendBoxedFuture<Result<Response>>
					},
				))
				.with_capabilities(Self::CAPABILITIES.to_vec())
				.xok()
			} else if #[cfg(feature = "sqlite")] {
				let dir = self.dir.clone().unwrap_or_else(|| {
					WsPath::new(format!("target/stores/{}", self.binding)).into()
				});
				let host = PartitionHost::new(dir);
				TableWireClient::new(TableWireTransport::new(
					format!("partition-host:{}", self.binding),
					move |request| {
						let host = host.clone();
						Box::pin(async move { host.serve(request).await.xok() })
							as SendBoxedFuture<Result<Response>>
					},
				))
				.with_capabilities(Self::CAPABILITIES.to_vec())
				.xok()
			} else {
				// refused at use, so a document declaring the store loads in
				// any build, a deploy's included
				let binding = self.binding.clone();
				TableWireClient::new(TableWireTransport::new(
					format!("durable-object:{binding}"),
					move |_request| {
						let binding = binding.clone();
						Box::pin(async move {
							bevybail!(
								"`DurableObjectStore` `{binding}` runs as Durable Objects in \
								 a Worker (a wasm build with `cloudflare`) or as SQLite \
								 files natively (the `sqlite` feature), and this build has \
								 neither"
							)
						}) as SendBoxedFuture<Result<Response>>
					},
				))
				.with_capabilities(Self::CAPABILITIES.to_vec())
				.xok()
			}
		}
	}

	/// Send `request` to the object of the partition its path names, the
	/// partition's segments stripped, so the object answers as its own root.
	#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
	async fn fetch(binding: &str, request: Request) -> Result<Response> {
		let (id, path) =
			TableWire::split_partition(request.request_parts().path())?;
		let name =
			id.map_or_else(|| Self::ROOT.to_string(), |id| id.to_string());
		let mut url = Url::new(
			Scheme::Https,
			Some("object".into()),
			path,
			default(),
			None,
		);
		for (key, value) in request.request_parts().params().iter_all() {
			for value in value {
				url = url.with_param(key.as_str(), value.as_str());
			}
		}
		let stub = WorkerEnv::with(|env| {
			env.durable_object(binding)?
				.id_from_name(&name)?
				.get_stub()
				.map_err(BevyError::from)
		})?;
		let request =
			WorkerEnv::to_worker_request(request, &url.to_string()).await?;
		WorkerEnv::to_response(stub.fetch_with_request(request).await?).await
	}

	/// Land the erased currencies: the [`TableStore`] over the client, and
	/// the [`BlobStore`] of the declaration's own partition.
	fn on_insert(mut world: DeferredWorld, cx: HookContext) {
		let store = world.entity(cx.entity).get::<Self>().cloned();
		match store.map(|store| store.client()) {
			Some(Ok(client)) => {
				world.commands().entity(cx.entity).insert((
					TableStore::new(client.clone()),
					BlobStore::new(client),
				));
			}
			Some(Err(err)) => {
				world.fallback_error_handler()(err, ErrorContext::Command {
					name: "DurableObjectStore::on_insert".into(),
				});
			}
			None => {}
		}
	}
}

/// One Durable Object's own SQLite storage as a partition: [`SqlTable`]'s
/// schema and semantics run through the object's `SqlStorage`, blobs in a
/// `blobs` table beside the rows. Every statement runs synchronously in the
/// object's event, which nothing interleaves, and a batch checks every
/// precondition before it writes ([`SqlTable::apply`]), the object's storage
/// refusing `BEGIN`.
///
/// An object is one partition, the Worker having routed every partition to
/// its own: [`partition`](TableProvider::partition) answers this one.
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
#[derive(Clone)]
pub struct DurableObjectSql {
	sql: worker::SqlStorage,
}

#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod object {
	use super::*;
	use bytes::Bytes;
	use send_wrapper::SendWrapper;
	use worker::SqlStorageValue;

	impl DurableObjectSql {
		/// The partition `sql` stores.
		pub fn new(sql: worker::SqlStorage) -> Self { Self { sql } }

		/// Run `func` against this storage as a provider future.
		fn run<T: 'static>(
			&self,
			func: impl 'static + FnOnce(&Self) -> Result<T>,
		) -> SendBoxedFuture<Result<T>> {
			let this = self.clone();
			Box::pin(SendWrapper::new(async move { func(&this) }))
		}

		fn bind(arg: &SqlArg) -> Result<SqlStorageValue> {
			match arg {
				SqlArg::Null => SqlStorageValue::Null,
				// past 2^53 a js number cannot hold it exactly: bound as a float
				SqlArg::Integer(int) => SqlStorageValue::try_from_i64(*int)
					.unwrap_or(SqlStorageValue::Float(*int as f64)),
				SqlArg::Real(float) => SqlStorageValue::Float(*float),
				SqlArg::Text(text) => SqlStorageValue::String(text.clone()),
				SqlArg::Blob(blob) => SqlStorageValue::Blob(blob.clone()),
			}
			.xok()
		}

		fn column(value: SqlStorageValue) -> SqlArg {
			match value {
				SqlStorageValue::Null => SqlArg::Null,
				SqlStorageValue::Boolean(bool) => SqlArg::Integer(bool as i64),
				SqlStorageValue::Integer(int) => SqlArg::Integer(int),
				SqlStorageValue::Float(float) => SqlArg::Real(float),
				SqlStorageValue::String(text) => SqlArg::Text(text),
				SqlStorageValue::Blob(blob) => SqlArg::Blob(blob),
			}
		}

		fn exec(
			&self,
			sql: &str,
			args: &[SqlArg],
		) -> Result<worker::SqlCursor> {
			let bindings =
				args.iter().map(Self::bind).collect::<Result<Vec<_>>>()?;
			self.sql.exec(sql, bindings)?.xok()
		}

		fn ensure_blobs(&self) -> Result {
			self.execute(
				"CREATE TABLE IF NOT EXISTS blobs (\
					path TEXT PRIMARY KEY, body BLOB NOT NULL, version TEXT NOT NULL)",
				&[],
			)
			.map(drop)
		}

		/// The blob at `path` with its version.
		fn blob(&self, path: &RelPath) -> Result<Option<(Bytes, BlobVersion)>> {
			self.ensure_blobs()?;
			match self
				.query("SELECT body, version FROM blobs WHERE path = ?1", &[
					SqlArg::Text(path.to_string()),
				])?
				.into_iter()
				.next()
			{
				Some(row) => match <[SqlArg; 2]>::try_from(row) {
					Ok([SqlArg::Blob(body), SqlArg::Text(version)]) => {
						Some((Bytes::from(body), BlobVersion::new(version)))
					}
					_ => bevybail!("blob `{path}` is not a body and a version"),
				},
				None => None,
			}
			.xok()
		}
	}

	impl SqlEngine for DurableObjectSql {
		fn execute(&self, sql: &str, args: &[SqlArg]) -> Result<usize> {
			let cursor = self.exec(sql, args)?;
			// the statement runs as its cursor is consumed
			for row in cursor.raw() {
				row?;
			}
			cursor.rows_written().xok()
		}

		fn query(
			&self,
			sql: &str,
			args: &[SqlArg],
		) -> Result<Vec<Vec<SqlArg>>> {
			self.exec(sql, args)?
				.raw()
				.map(|row| Ok(row?.into_iter().map(Self::column).collect()))
				.collect()
		}

		/// The object's storage refuses `BEGIN`, and its event is never
		/// interleaved: the batch runs as it is, every precondition already
		/// checked ([`SqlTable::apply`]).
		fn transaction(
			&self,
			func: &mut dyn FnMut(&dyn SqlEngine) -> Result,
		) -> Result {
			func(self)
		}
	}

	impl BlobStoreProvider for DurableObjectSql {
		fn box_clone(&self) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn with_subdir(&self, _path: RelPath) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn base(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }
		fn id(&self) -> &'static str { "durable-object" }
		fn root_key(&self) -> SmolStr { "durable-object".into() }
		fn region(&self) -> Option<String> { None }
		fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { true.xok() })
		}
		fn store_create(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		/// Drop every table, the object's storage left empty.
		fn store_remove(&self) -> SendBoxedFuture<Result> {
			self.run(|this| {
				let tables = this.query(
					"SELECT name FROM sqlite_master WHERE type = 'table' \
					 AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%'",
					&[],
				)?;
				for table in tables {
					if let Some(SqlArg::Text(name)) = table.into_iter().next() {
						this.execute(
							&format!("DROP TABLE IF EXISTS \"{name}\""),
							&[],
						)?;
					}
				}
				Ok(())
			})
		}
		fn insert(
			&self,
			path: &RelPath,
			body: Bytes,
		) -> SendBoxedFuture<Result> {
			let insert = self.insert_if(path, body, BlobCondition::Any);
			Box::pin(async move { insert.await.map(drop) })
		}
		fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
			self.run(|this| {
				this.ensure_blobs()?;
				this.query("SELECT path FROM blobs ORDER BY path", &[])?
					.into_iter()
					.map(|row| match row.into_iter().next() {
						Some(SqlArg::Text(path)) => RelPath::new(path).xok(),
						_ => bevybail!("blob path is not text"),
					})
					.collect()
			})
		}
		fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
			let path = path.clone();
			self.run(move |this| match this.blob(&path)? {
				Some((bytes, _)) => bytes.xok(),
				None => Err(HttpError::new(
					StatusCode::NOT_FOUND,
					format!("blob `{path}` not found"),
				)
				.into()),
			})
		}
		fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
			let path = path.clone();
			self.run(move |this| this.blob(&path).map(|blob| blob.is_some()))
		}
		fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
			let path = path.clone();
			self.run(move |this| {
				match this.execute("DELETE FROM blobs WHERE path = ?1", &[
					SqlArg::Text(path.to_string()),
				])? {
					0 => bevybail!("blob `{path}` not found"),
					_ => Ok(()),
				}
			})
		}
		fn get_versioned(
			&self,
			path: &RelPath,
		) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
			let path = path.clone();
			self.run(move |this| this.blob(&path))
		}
		fn insert_if(
			&self,
			path: &RelPath,
			body: Bytes,
			condition: BlobCondition,
		) -> SendBoxedFuture<Result<BlobVersion>> {
			let path = path.clone();
			self.run(move |this| {
				let found = this.blob(&path)?;
				BlobConflict::check(&condition, &path, found.as_ref().map(|(_, version)| version))?;
				let version = BlobVersion::of(&body);
				this.execute(
					"INSERT INTO blobs (path, body, version) VALUES (?1, ?2, ?3) \
					 ON CONFLICT(path) DO UPDATE SET body = excluded.body, version = excluded.version",
					&[
						SqlArg::Text(path.to_string()),
						SqlArg::Blob(body.to_vec()),
						SqlArg::Text(version.to_string()),
					],
				)?;
				version.xok()
			})
		}
		fn remove_if(
			&self,
			path: &RelPath,
			condition: BlobCondition,
		) -> SendBoxedFuture<Result> {
			let path = path.clone();
			self.run(move |this| {
				let found = this.blob(&path)?;
				BlobConflict::check(
					&condition,
					&path,
					found.as_ref().map(|(_, version)| version),
				)?;
				this.execute("DELETE FROM blobs WHERE path = ?1", &[
					SqlArg::Text(path.to_string()),
				])?;
				Ok(())
			})
		}
		fn public_url(
			&self,
			_path: &RelPath,
		) -> SendBoxedFuture<Result<Option<String>>> {
			Box::pin(async { None.xok() })
		}
	}

	impl TableProvider for DurableObjectSql {
		fn box_clone_table(&self) -> Box<dyn TableProvider> {
			Box::new(self.clone())
		}
		fn partition(&self, _id: &PartitionId) -> Box<dyn TableProvider> {
			Box::new(self.clone())
		}
		fn supports(&self, capability: Capability) -> bool {
			DurableObjectStore::CAPABILITIES.contains(&capability)
		}
		fn get_row(
			&self,
			table: &TableSpec,
			key: &TableKey,
		) -> SendBoxedFuture<Result<Option<StoredRow>>> {
			let table = table.clone();
			let key = key.clone();
			self.run(move |this| SqlTable::new(&table)?.get(this, &key))
		}
		fn put_row(
			&self,
			table: &TableSpec,
			row: StoredRow,
			precondition: Precondition,
		) -> SendBoxedFuture<Result> {
			let table = table.clone();
			self.run(move |this| {
				SqlTable::new(&table)?.put(this, &row, &precondition)
			})
		}
		fn remove_row(
			&self,
			table: &TableSpec,
			key: &TableKey,
			precondition: Precondition,
		) -> SendBoxedFuture<Result> {
			let table = table.clone();
			let key = key.clone();
			self.run(move |this| {
				SqlTable::new(&table)?.remove(this, &key, &precondition)
			})
		}
		fn scan_rows(
			&self,
			table: &TableSpec,
			scan: Scan,
		) -> SendBoxedFuture<Result<ScanPage>> {
			let table = table.clone();
			self.run(move |this| SqlTable::new(&table)?.scan(this, &scan))
		}
		fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
			self.run(move |this| SqlTable::apply(this, &writes))
		}
		fn scan_index(
			&self,
			table: &TableSpec,
			index: &TableIndex,
			scan: IndexScan,
		) -> SendBoxedFuture<Result<ScanPage>> {
			let table = table.clone();
			let index = index.clone();
			self.run(move |this| {
				SqlTable::new(&table)?.scan_index(this, &index, &scan)
			})
		}
	}

	/// The Durable Object class every partition of a [`DurableObjectStore`]
	/// is: it answers the [`TableWire`] protocol over its own SQLite storage,
	/// exported from the Worker's wasm under [`DurableObjectStore::CLASS`].
	#[worker::durable_object(fetch)]
	pub struct BeetTableObject {
		state: worker::State,
	}

	impl worker::DurableObject for BeetTableObject {
		fn new(state: worker::State, _env: worker::Env) -> Self {
			Self { state }
		}

		async fn fetch(
			&self,
			req: worker::Request,
		) -> worker::Result<worker::Response> {
			let store = DurableObjectSql::new(self.state.storage().sql());
			let response = match WorkerEnv::to_request(req).await {
				Ok(request) => {
					let path = request.request_parts().path().clone();
					TableWire::answer(&store, &path, request).await
				}
				Err(err) => Response::status_text(
					StatusCode::BAD_REQUEST,
					err.to_string(),
				),
			};
			WorkerEnv::to_worker_response(response)
				.await
				.map_err(|err| worker::Error::RustError(err.to_string()))
		}
	}
}

#[cfg(all(test, feature = "sqlite", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Natively the declaration is a partition host: its currencies land on
	/// the entity, and each partition is a file under its directory.
	#[beet_core::test]
	async fn resolves_to_a_partition_host() {
		let dir = TempDir::new_ws().unwrap();
		let mut world = World::new();
		let entity = world
			.spawn(
				DurableObjectStore::new("TABLES").with_dir(dir.path().clone()),
			)
			.id();
		world.flush();
		let store = world.entity(entity).get::<TableStore>().unwrap().clone();
		store.supports(Capability::AtomicApply).xpect_true();
		store
			.partition("lobby")
			.unwrap()
			.table::<table_test::NamedRow>()
			.put(table_test::NamedRow::new("k", 1))
			.await
			.unwrap();
		fs_ext::exists(dir.path().join("root/lobby.db"))
			.unwrap()
			.xpect_true();
	}
}
