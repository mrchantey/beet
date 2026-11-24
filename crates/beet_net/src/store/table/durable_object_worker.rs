//! The Worker side of a [`DurableObjectStore`]: the client sending each
//! partition's requests to its object, and the object answering them over its
//! own SQLite storage.
use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use send_wrapper::SendWrapper;
use worker::SqlStorageValue;

impl DurableObjectStore {
	/// The client sending each request to the object of the partition its
	/// path names, through the namespace `binding`.
	pub(crate) fn object_client(binding: SmolStr) -> TableWireClient {
		TableWireClient::new(TableWireTransport::new(
			format!("durable-object:{binding}"),
			move |request| {
				let binding = binding.clone();
				Box::pin(SendWrapper::new(async move {
					Self::fetch(&binding, request).await
				})) as SendBoxedFuture<Result<Response>>
			},
		))
		.with_capabilities(Self::CAPABILITIES.to_vec())
	}

	/// Send `request` to the object of the partition its path names, the
	/// partition's segments stripped, so the object answers as its own root.
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
#[derive(Clone)]
pub struct DurableObjectSql {
	sql: worker::SqlStorage,
}

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

	fn exec(&self, sql: &str, args: &[SqlArg]) -> Result<worker::SqlCursor> {
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

	fn query(&self, sql: &str, args: &[SqlArg]) -> Result<Vec<Vec<SqlArg>>> {
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
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }
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
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
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

// private, so the class is the Worker's export alone: `wasm_bindgen` exports
// it by symbol, and its macro's glue never reaches the crate's api
mod class {
	use super::*;

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
