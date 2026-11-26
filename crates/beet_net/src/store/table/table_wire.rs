//! The table wire protocol: one partition's rows over beet's
//! [`Request`]/[`Response`], so a table store is reachable from another
//! process exactly as it is in this one.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use core::ops::Bound;
use serde::Deserialize;
use serde::Serialize;

/// The table wire protocol, served by [`TableWire::serve`] and spoken by
/// [`TableWireClient`]: a server facet over http (`<ServeTables/>`), a Durable
/// Object over its stub fetch and the in-process [`PartitionHost`] all answer
/// it, and one client reads all three.
///
/// Every path is relative to the mount. A partition is a leading `@<id>`
/// segment per partition id segment (no table name begins with `@`), so
/// `/@did:plc:abc/notes/3mw72aaeuj22n` is a row of partition `did:plc:abc`.
/// Rows travel as [`StoredRow`] json, cid included, never recomputed.
///
/// | request | answer |
/// | --- | --- |
/// | `GET /<table>/<key>` | the row, `ETag: "<cid>"`; `404` for none |
/// | `PUT /<table>/<key>` `{row, indexes}` | `204`, `ETag: "<cid>"` |
/// | `DELETE /<table>/<key>` | `204` |
/// | `GET /<table>?from=&after=&through=&before=&limit=&reverse&cursor=` | a [`ScanPage`] |
/// | `POST /_scan_index` `{table, index, scan}` | a [`ScanPage`] |
/// | `POST /_apply` `{writes}` | `204`, every write or none |
/// | `GET /_capabilities` | the [`Capability`]s supported |
/// | `GET` / `POST` / `DELETE` `/_store` | exists (`200`/`404`), create, remove |
///
/// Rows alone: a mount of bytes is `<ServeBlobs/>`. A write's precondition
/// rides its headers: `If-Match: "<cid>"` or `If-None-Match: *`. A failed one
/// answers `412` with the [`TableError::Conflict`] as json, an unsupported
/// capability `501` and a limit `413`, each read back by the client as the
/// same typed error.
pub struct TableWire;

impl TableWire {
	/// The reserved segment of an atomic batch.
	const APPLY: &'static str = "_apply";
	/// The reserved segment of an index scan.
	const SCAN_INDEX: &'static str = "_scan_index";
	/// The reserved segment of the capability listing.
	const CAPABILITIES: &'static str = "_capabilities";
	/// The reserved segment of the store's lifecycle.
	const STORE: &'static str = "_store";

	/// Answer `request`, its path relative to the mount, from `provider`.
	/// Never fails: an error is the response that says so.
	pub async fn serve(
		provider: &dyn TableProvider,
		request: Request,
	) -> Response {
		let path = request.request_parts().path().clone();
		Self::serve_path(provider, &path, request).await
	}

	/// [`serve`](Self::serve) `request` as though its path were `path`, the
	/// part below a mount: what a route serving the protocol under a prefix
	/// answers with.
	pub async fn serve_path(
		provider: &dyn TableProvider,
		path: &[SmolStr],
		request: Request,
	) -> Response {
		match Self::split_partition(path) {
			Ok((Some(id), path)) => {
				Self::answer(provider.partition(&id).as_ref(), &path, request)
					.await
			}
			Ok((None, path)) => Self::answer(provider, &path, request).await,
			Err(err) => Self::error_response(err),
		}
	}

	/// The partition a path's leading `@` segments name, `None` for the
	/// declaration's own, and the path after them.
	pub fn split_partition(
		path: &[SmolStr],
	) -> Result<(Option<PartitionId>, Vec<SmolStr>)> {
		let ids = path
			.iter()
			.map_while(|segment| segment.strip_prefix('@'))
			.collect::<Vec<_>>();
		let rest = path[ids.len()..].to_vec();
		match ids.is_empty() {
			true => (None, rest),
			false => (Some(PartitionId::parse(&ids.join("/"))?), rest),
		}
		.xok()
	}

	/// Answer `request` from `provider`, the partition already chosen and
	/// `path` the request's path after it.
	pub async fn answer(
		provider: &dyn TableProvider,
		path: &[SmolStr],
		request: Request,
	) -> Response {
		let method = *request.method();
		match Self::route(provider, method, path, request).await {
			Ok(response) => response,
			Err(err) => Self::error_response(err),
		}
	}

	async fn route(
		provider: &dyn TableProvider,
		method: HttpMethod,
		path: &[SmolStr],
		request: Request,
	) -> Result<Response> {
		let precondition =
			Self::precondition(request.request_parts().headers())?;
		match (method, path) {
			(HttpMethod::Get, [segment]) if segment == Self::CAPABILITIES => {
				Capability::ALL
					.into_iter()
					.filter(|capability| provider.supports(*capability))
					.collect::<Vec<_>>()
					.xref()
					.xmap(Response::with_json)
			}
			(_, [segment]) if segment == Self::STORE => match method {
				HttpMethod::Get => match provider.store_exists().await? {
					true => Response::ok(),
					false => Response::not_found(),
				}
				.xok(),
				HttpMethod::Post => {
					provider.store_create().await?;
					Response::from_status(StatusCode::NO_CONTENT).xok()
				}
				HttpMethod::Delete => {
					provider.store_remove().await?;
					Response::from_status(StatusCode::NO_CONTENT).xok()
				}
				_ => Self::method_not_allowed(),
			},
			(HttpMethod::Post, [segment]) if segment == Self::APPLY => {
				let body = Self::body::<ApplyBody>(request).await?;
				TableStore::from_box(provider.box_clone_table())
					.apply(body.writes)
					.await?;
				Response::from_status(StatusCode::NO_CONTENT).xok()
			}
			(HttpMethod::Post, [segment]) if segment == Self::SCAN_INDEX => {
				let body = Self::body::<ScanIndexBody>(request).await?;
				let index = body.table.index(&body.index)?.clone();
				provider
					.scan_index(&body.table, &index, body.scan)
					.await?
					.xref()
					.xmap(Response::with_json)
			}
			(HttpMethod::Get, [table]) => {
				let scan = Self::scan_of(request.request_parts())?;
				provider
					.scan_rows(&TableSpec::new(table.clone()), scan)
					.await?
					.xref()
					.xmap(Response::with_json)
			}
			(_, [table, key @ ..]) if !key.is_empty() => {
				let key = TableKey::from(key.join("/"));
				let spec = TableSpec::new(table.clone());
				match method {
					HttpMethod::Get => {
						match provider.get_row(&spec, &key).await? {
							Some(row) => Response::with_json(&row)?
								.with_header("etag", &Self::etag(row.cid())),
							None => Response::not_found(),
						}
						.xok()
					}
					HttpMethod::Put => {
						let body = Self::body::<PutBody>(request).await?;
						if body.row.key() != &key {
							bevybail!(
								"row `{}` was put at key `{key}`",
								body.row.key()
							);
						}
						let cid = body.row.cid().clone();
						let spec = spec.with_indexes(body.indexes);
						TableStore::from_box(provider.box_clone_table())
							.put(spec, body.row, precondition)
							.await?;
						Response::from_status(StatusCode::NO_CONTENT)
							.with_header("etag", &Self::etag(&cid))
							.xok()
					}
					HttpMethod::Delete => {
						provider.remove_row(&spec, &key, precondition).await?;
						Response::from_status(StatusCode::NO_CONTENT).xok()
					}
					_ => Self::method_not_allowed(),
				}
			}
			_ => Response::not_found().xok(),
		}
	}

	/// The quoted etag of a cid.
	fn etag(cid: &Cid) -> String { format!("\"{cid}\"") }

	/// The precondition a request's headers carry.
	fn precondition(headers: &HeaderMap) -> Result<Precondition> {
		match (
			headers.first_raw("if-match"),
			headers.first_raw("if-none-match"),
		) {
			(Some(_), Some(_)) => {
				bevybail!(
					"a write takes `If-Match` or `If-None-Match`, not both"
				)
			}
			(Some(tag), None) => {
				Precondition::Matches(Cid::parse(tag.trim_matches('"'))?)
			}
			(None, Some("*")) => Precondition::Absent,
			(None, Some(other)) => {
				bevybail!("`If-None-Match: {other}` is not supported, only `*`")
			}
			(None, None) => Precondition::Any,
		}
		.xok()
	}

	/// The scan a row listing's query params ask for.
	fn scan_of(parts: &RequestParts) -> Result<Scan> {
		let mut scan = Scan::default();
		let key = |name: &str| parts.get_param(name).map(TableKey::from);
		if let Some(key) = key("from") {
			scan.start = Bound::Included(key);
		}
		if let Some(key) = key("after") {
			scan.start = Bound::Excluded(key);
		}
		if let Some(key) = key("through") {
			scan.end = Bound::Included(key);
		}
		if let Some(key) = key("before") {
			scan.end = Bound::Excluded(key);
		}
		if let Some(limit) = parts.get_param("limit") {
			scan.limit = Some(limit.parse()?);
		}
		scan.reverse = parts.get_param("reverse").is_some();
		scan.cursor = parts.get_param("cursor").map(ScanCursor::new);
		scan.xok()
	}

	/// The query params a scan is sent as, [`scan_of`](Self::scan_of)'s
	/// inverse.
	fn scan_params(scan: &Scan) -> Vec<(&'static str, String)> {
		let mut params = Vec::new();
		match &scan.start {
			Bound::Included(key) => params.push(("from", key.to_string())),
			Bound::Excluded(key) => params.push(("after", key.to_string())),
			Bound::Unbounded => {}
		}
		match &scan.end {
			Bound::Included(key) => params.push(("through", key.to_string())),
			Bound::Excluded(key) => params.push(("before", key.to_string())),
			Bound::Unbounded => {}
		}
		if let Some(limit) = scan.limit {
			params.push(("limit", limit.to_string()));
		}
		if scan.reverse {
			params.push(("reverse", "true".into()));
		}
		if let Some(cursor) = &scan.cursor {
			params.push(("cursor", cursor.to_string()));
		}
		params
	}

	async fn body<T: serde::de::DeserializeOwned>(
		request: Request,
	) -> Result<T> {
		let bytes = request.into_parts().1.into_bytes().await?;
		serde_json::from_slice::<T>(&bytes)?.xok()
	}

	fn method_not_allowed() -> Result<Response> {
		Response::from_status(StatusCode::METHOD_NOT_ALLOWED).xok()
	}

	/// The response an error is: a table error as its status and json, an
	/// http error its own status, anything else `500`.
	fn error_response(err: BevyError) -> Response {
		if let Some(table) = TableError::of(&err) {
			let status = match table {
				TableError::Conflict { .. } => StatusCode::PRECONDITION_FAILED,
				TableError::Unsupported { .. } => StatusCode::NOT_IMPLEMENTED,
				TableError::Limit { .. } => StatusCode::CONTENT_TOO_LARGE,
			};
			return Response::with_json(table)
				.map(|response| response.with_status(status))
				.unwrap_or_else(|_| Response::from_status(status));
		}
		if let Some(http) = err.downcast_ref::<HttpError>() {
			return Response::status_text(
				http.status_code,
				http.message.clone(),
			);
		}
		Response::status_text(
			StatusCode::INTERNAL_SERVER_ERROR,
			err.to_string(),
		)
	}
}

/// `PUT /<table>/<key>`.
#[derive(Serialize, Deserialize)]
struct PutBody {
	row: StoredRow,
	#[serde(default)]
	indexes: Vec<TableIndex>,
}

/// `POST /_apply`.
#[derive(Serialize, Deserialize)]
struct ApplyBody {
	writes: Vec<TableWrite>,
}

/// `POST /_scan_index`.
#[derive(Serialize, Deserialize)]
struct ScanIndexBody {
	table: TableSpec,
	index: SmolStr,
	scan: IndexScan,
}

/// Where a [`TableWireClient`]'s requests go: an http server, or a function
/// answering them in process (a [`PartitionHost`], a Durable Object's stub
/// fetch).
#[derive(Clone)]
pub struct TableWireTransport {
	/// The mount, its path the prefix every request's path is joined to.
	base: Url,
	/// What sends a request.
	send:
		Arc<dyn Fn(Request) -> SendBoxedFuture<Result<Response>> + Send + Sync>,
	/// Where requests go, for any error naming it.
	describe: SmolStr,
}

impl TableWireTransport {
	/// Requests to the server mounted at `base`, ie
	/// `http://127.0.0.1:8337/tables`, sent with [`Request::send`], whose
	/// future is `Send` where an http client is.
	#[cfg(feature = "std")]
	pub fn http(base: impl AsRef<str>) -> Result<Self> {
		let base = Url::parse(base.as_ref())?;
		Self {
			describe: format!("table-wire:{base}").into(),
			base,
			send: Arc::new(|request: Request| {
				Box::pin(async move { request.send().await })
					as SendBoxedFuture<Result<Response>>
			}),
		}
		.xok()
	}

	/// Requests answered by `send` in process, its paths rooted at `/`.
	pub fn new(
		describe: impl Into<SmolStr>,
		send: impl 'static
		+ Send
		+ Sync
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
	) -> Self {
		Self {
			base: Url::parse("/").expect("a root path parses"),
			send: Arc::new(send),
			describe: describe.into(),
		}
	}

	/// The url of `segments` below the mount.
	fn url(
		&self,
		segments: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Url {
		let mut path = self.base.path().clone();
		path.extend(segments.into_iter().map(Into::into));
		Url::new(
			self.base.scheme().clone(),
			self.base.authority().map(SmolStr::from),
			path,
			default(),
			None,
		)
	}
}

/// A [`TableProvider`] speaking the [`TableWire`] protocol to wherever its
/// [`TableWireTransport`] sends: another process's served store over http,
/// the in-process [`PartitionHost`], or a Durable Object.
///
/// Its capabilities are the server's, fetched once and remembered.
#[derive(Clone)]
pub struct TableWireClient {
	transport: TableWireTransport,
	partition: Option<PartitionId>,
	capabilities: Arc<bevy::platform::sync::OnceLock<Vec<Capability>>>,
}

impl core::fmt::Debug for TableWireClient {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("TableWireClient")
			.field("transport", &self.transport.describe)
			.field("partition", &self.partition)
			.finish()
	}
}

impl TableWireClient {
	/// A client of the store `transport` reaches, its capabilities unknown
	/// until fetched: prefer [`connect`](Self::connect).
	pub fn new(transport: TableWireTransport) -> Self {
		Self {
			transport,
			partition: None,
			capabilities: default(),
		}
	}

	/// A client of the store `transport` reaches, its capabilities fetched so
	/// [`supports`](TableProvider::supports) answers them.
	pub async fn connect(transport: TableWireTransport) -> Result<Self> {
		let client = Self::new(transport);
		client.capabilities().await?;
		client.xok()
	}

	/// The same client, its capabilities known without asking: a client of a
	/// backend whose capabilities are fixed.
	pub fn with_capabilities(self, capabilities: Vec<Capability>) -> Self {
		let _ = self.capabilities.set(capabilities);
		self
	}

	/// The capabilities the server answered, fetched once.
	pub async fn capabilities(&self) -> Result<Vec<Capability>> {
		if let Some(capabilities) = self.capabilities.get() {
			return capabilities.clone().xok();
		}
		let response = self
			.send(Request::get(self.url([TableWire::CAPABILITIES])))
			.await?;
		let capabilities = Self::checked(response)
			.await?
			.json::<Vec<Capability>>()
			.await?;
		let _ = self.capabilities.set(capabilities.clone());
		capabilities.xok()
	}

	/// The url of `segments` below this partition.
	fn url(
		&self,
		segments: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Url {
		let partition = self
			.partition
			.iter()
			.flat_map(|id| {
				id.segments()
					.map(|segment| SmolStr::from(format!("@{segment}")))
			})
			.collect::<Vec<_>>();
		self.transport.url(
			partition
				.into_iter()
				.chain(segments.into_iter().map(Into::into)),
		)
	}

	/// The url of a row.
	fn row_url(&self, table: &TableSpec, key: &TableKey) -> Url {
		self.url([table.name.clone(), SmolStr::from(key.as_str())])
	}

	async fn send(&self, request: Request) -> Result<Response> {
		(self.transport.send)(request).await
	}

	/// `request` with `precondition` as its headers.
	fn with_precondition(
		request: Request,
		precondition: &Precondition,
	) -> Request {
		match precondition {
			Precondition::Any => request,
			Precondition::Absent => {
				request.with_header_raw("if-none-match", "*")
			}
			Precondition::Matches(cid) => {
				request.with_header_raw("if-match", &TableWire::etag(cid))
			}
		}
	}

	/// The response when it succeeded, else the typed error it carries.
	async fn checked(response: Response) -> Result<Response> {
		let status = response.status();
		if status.is_success() {
			return Ok(response);
		}
		let text = response.text().await.unwrap_or_default();
		match serde_json::from_str::<TableError>(&text) {
			Ok(err) => Err(err.into()),
			Err(_) => Err(HttpError::new(status, text).into()),
		}
	}
}

impl StoreBackend for TableWireClient {
	fn id(&self) -> &'static str { "table-wire" }

	fn describe(&self) -> String {
		match &self.partition {
			Some(partition) => {
				format!("{}/@{partition}", self.transport.describe)
			}
			None => self.transport.describe.to_string(),
		}
	}

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move {
			let response = this
				.send(Request::get(this.url([TableWire::STORE])))
				.await?;
			match response.status() {
				status if status.is_success() => Ok(true),
				StatusCode::NOT_FOUND => Ok(false),
				_ => Self::checked(response).await.map(|_| false),
			}
		})
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			Self::checked(
				this.send(Request::post(this.url([TableWire::STORE])))
					.await?,
			)
			.await?;
			Ok(())
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			Self::checked(
				this.send(Request::delete(this.url([TableWire::STORE])))
					.await?,
			)
			.await?;
			Ok(())
		})
	}
}

impl TableProvider for TableWireClient {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider> {
		Box::new(Self {
			partition: Some(match &self.partition {
				Some(parent) => parent.join(id),
				None => id.clone(),
			}),
			// the same server, so the same capabilities
			capabilities: self.capabilities.clone(),
			transport: self.transport.clone(),
		})
	}

	/// The capabilities the server answered, none before they are fetched
	/// ([`TableWireClient::connect`]).
	fn supports(&self, capability: Capability) -> bool {
		self.capabilities
			.get()
			.is_some_and(|capabilities| capabilities.contains(&capability))
	}

	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>> {
		let this = self.clone();
		let url = self.row_url(table, key);
		Box::pin(async move {
			let response = this.send(Request::get(url)).await?;
			if response.status() == StatusCode::NOT_FOUND {
				return Ok(None);
			}
			Self::checked(response)
				.await?
				.json::<StoredRow>()
				.await?
				.xsome()
				.xok()
		})
	}

	fn put_row(
		&self,
		table: &TableSpec,
		row: StoredRow,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let url = self.row_url(table, row.key());
		let body = PutBody {
			row,
			indexes: table.indexes.clone(),
		};
		Box::pin(async move {
			let request = Self::with_precondition(
				Request::put(url).with_body(serde_json::to_vec(&body)?),
				&precondition,
			);
			Self::checked(this.send(request).await?).await.map(drop)
		})
	}

	fn remove_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let url = self.row_url(table, key);
		Box::pin(async move {
			let request =
				Self::with_precondition(Request::delete(url), &precondition);
			Self::checked(this.send(request).await?).await.map(drop)
		})
	}

	fn scan_rows(
		&self,
		table: &TableSpec,
		scan: Scan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let mut request = Request::get(self.url([table.name.clone()]));
		for (name, value) in TableWire::scan_params(&scan) {
			request = request.with_param(name, &value);
		}
		Box::pin(async move {
			Self::checked(this.send(request).await?)
				.await?
				.json::<ScanPage>()
				.await
		})
	}

	/// Sent whatever the capabilities, so the server refuses or applies it.
	fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let url = self.url([TableWire::APPLY]);
		Box::pin(async move {
			let body = serde_json::to_vec(&ApplyBody { writes })?;
			Self::checked(this.send(Request::post(url).with_body(body)).await?)
				.await
				.map(drop)
		})
	}

	fn scan_index(
		&self,
		table: &TableSpec,
		index: &TableIndex,
		scan: IndexScan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let url = self.url([TableWire::SCAN_INDEX]);
		let body = ScanIndexBody {
			table: table.clone(),
			index: index.name.clone(),
			scan,
		};
		Box::pin(async move {
			let body = serde_json::to_vec(&body)?;
			Self::checked(this.send(Request::post(url).with_body(body)).await?)
				.await?
				.json::<ScanPage>()
				.await
		})
	}

	fn expire_rows(
		&self,
		table: &TableSpec,
		keys: Vec<TableKey>,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move {
			let writes = keys
				.into_iter()
				.map(|key| TableWrite::Remove {
					table: table.clone(),
					key,
					precondition: Precondition::Any,
				})
				.collect::<Vec<_>>();
			match this
				.capabilities()
				.await?
				.contains(&Capability::AtomicApply)
			{
				true => this.apply(writes).await,
				false => {
					for write in writes {
						this.remove_row(
							write.table(),
							write.key(),
							Precondition::Any,
						)
						.await?;
					}
					Ok(())
				}
			}
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn json_tables(blobs: BlobStore) -> JsonTableProvider {
		JsonTableProvider::new(blobs).unwrap()
	}

	/// The suite through the protocol, answered in process by `provider`.
	async fn in_process(provider: impl TableProvider + Clone) {
		let client = TableWireClient::connect(TableWireTransport::new(
			"in-process",
			move |request| {
				let provider = provider.clone();
				Box::pin(async move {
					TableWire::serve(&provider, request).await.xok()
				}) as SendBoxedFuture<Result<Response>>
			},
		))
		.await
		.unwrap();
		table_test::run(TableStore::new(client)).await;
	}

	#[beet_core::test]
	async fn json() { in_process(json_tables(BlobStore::temp())).await }

	#[cfg(feature = "sqlite")]
	#[beet_core::test(timeout_ms = 60_000)]
	async fn sqlite() { in_process(SqliteStore::memory("wire-sqlite")).await }

	/// A key holding a `/`, `#`, `?`, a space and multi-byte text reaches the
	/// server whole.
	#[beet_core::test]
	async fn keys_survive_the_path() {
		let provider = json_tables(BlobStore::temp());
		let store = TableStore::new(
			TableWireClient::connect(TableWireTransport::new(
				"in-process",
				move |request| {
					let provider = provider.clone();
					Box::pin(async move {
						TableWire::serve(&provider, request).await.xok()
					}) as SendBoxedFuture<Result<Response>>
				},
			))
			.await
			.unwrap(),
		);
		let rows = store.table::<table_test::NamedRow>();
		for key in ["a/b", "c#d", "e?f", "g h", "日本"] {
			rows.put(table_test::NamedRow::new(key, 1)).await.unwrap();
			rows.get(key).await.unwrap().unwrap().key.xpect_eq(key);
		}
	}

	/// The suite over http against a local server answering from each
	/// native backend.
	#[cfg(all(
		feature = "server",
		feature = "ureq",
		not(target_arch = "wasm32")
	))]
	mod http {
		use super::*;

		async fn over_http(provider: impl TableProvider + Clone) {
			let url = HttpServer::spawn_test(exchange_ext::handler_async(
				move |request: Request| {
					let provider = provider.clone();
					async move { TableWire::serve(&provider, request).await }
				},
			))
			.await;
			let client = TableWireClient::connect(
				TableWireTransport::http(&url).unwrap(),
			)
			.await
			.unwrap();
			table_test::run(TableStore::new(client)).await;
		}

		#[beet_core::test(timeout_ms = 60_000)]
		async fn json_memory() {
			over_http(json_tables(BlobStore::temp())).await
		}

		#[beet_core::test(timeout_ms = 60_000)]
		async fn json_fs() {
			let dir = TempDir::new_ws().unwrap();
			over_http(json_tables(BlobStore::new(FsStore::new(
				dir.path().clone(),
			))))
			.await
		}

		#[cfg(feature = "sqlite")]
		#[beet_core::test(timeout_ms = 120_000)]
		async fn sqlite() {
			let dir = TempDir::new_ws().unwrap();
			over_http(SqliteStore::new(dir.join("wire.db"))).await
		}

		/// The suite over http through a Worker under `wrangler dev`, each
		/// partition a Durable Object: a document mounting
		/// `<DurableObjectStore binding="TABLES"><ServeTables prefix="tables"/></DurableObjectStore>`,
		/// the Worker built with `worker-build` (`--features cloudflare,sqlite`).
		#[beet_core::test(timeout_ms = 600_000)]
		#[ignore = "needs a Worker under wrangler dev on 127.0.0.1:8787"]
		async fn durable_objects() {
			let client = TableWireClient::connect(
				TableWireTransport::http("http://127.0.0.1:8787/tables")
					.unwrap(),
			)
			.await
			.unwrap();
			table_test::run(TableStore::new(client)).await;
		}

		/// The suite's row cases over http through a Worker under `wrangler
		/// dev`, the rows json files on its R2 binding, so every write is the
		/// binding's conditional put: a document mounting
		/// `<R2WorkersStore binding="ARCHIVE" {JsonTableStore{path:"tables"}}><ServeTables prefix="r2"/></R2WorkersStore>`.
		#[beet_core::test(timeout_ms = 600_000)]
		#[ignore = "needs a Worker under wrangler dev on 127.0.0.1:8787"]
		async fn r2_binding() {
			let client = TableWireClient::connect(
				TableWireTransport::http("http://127.0.0.1:8787/r2").unwrap(),
			)
			.await
			.unwrap();
			table_test::run_rows(&TableStore::new(client)).await;
		}

		#[cfg(feature = "sqlite")]
		#[beet_core::test(timeout_ms = 120_000)]
		async fn partition_host() {
			let dir = TempDir::new_ws().unwrap();
			let host = PartitionHost::new(dir.path().clone());
			let url = HttpServer::spawn_test(exchange_ext::handler_async(
				move |request: Request| {
					let host = host.clone();
					async move { host.serve(request).await }
				},
			))
			.await;
			let client = TableWireClient::connect(
				TableWireTransport::http(&url).unwrap(),
			)
			.await
			.unwrap();
			table_test::run(TableStore::new(client)).await;
		}
	}
}
