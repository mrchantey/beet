//! A real PDS over xrpc.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bytes::Bytes;

/// A [`PdsProvider`] over a real PDS: the five xrpc calls that write
/// (`putRecord`, `deleteRecord`, `uploadBlob`, and the session pair behind
/// [`AtprotoAuth`]) and the two that read unauthenticated (`getRecord`,
/// `listRecords`).
///
/// The PDS is resolved from the did's document on first use and remembered,
/// so constructing one costs nothing; [`with_endpoint`](Self::with_endpoint)
/// pins it instead. Reads need no credential. A write with none is an error
/// naming the missing `secret`, and a write the server refuses as expired is
/// retried once after the credential recovers.
#[derive(Clone)]
pub struct XrpcPds {
	did: Did,
	endpoint: Arc<RwLock<Option<SmolStr>>>,
	resolver: DidResolver,
	auth: Option<Arc<dyn AtprotoAuth>>,
}

impl core::fmt::Debug for XrpcPds {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("XrpcPds")
			.field("did", &self.did)
			.field("endpoint", &self.pinned())
			.field("authenticated", &self.auth.is_some())
			.finish()
	}
}

impl XrpcPds {
	/// The provider id.
	pub const ID: &'static str = "xrpc";
	/// The page size `listRecords` allows at most.
	const PAGE: &'static str = "100";

	/// The repo of `did`, its PDS resolved on first use, read only until it
	/// is given a credential.
	pub fn new(did: Did) -> Self {
		Self {
			did,
			endpoint: default(),
			resolver: default(),
			auth: None,
		}
	}

	/// Pin the PDS rather than resolving it, ie a self-hosted or local one.
	pub fn with_endpoint(self, endpoint: impl AsRef<str>) -> Self {
		*self
			.endpoint
			.write()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner) =
			Some(SmolStr::new(endpoint.as_ref().trim_end_matches('/')));
		self
	}

	/// Resolve dids through `resolver` rather than the public directory.
	pub fn with_resolver(mut self, resolver: DidResolver) -> Self {
		self.resolver = resolver;
		self
	}

	/// Write with `auth`; `None` keeps the repo read only.
	pub fn with_auth(mut self, auth: Option<impl AtprotoAuth>) -> Self {
		self.auth = auth.map(|auth| Arc::new(auth) as Arc<dyn AtprotoAuth>);
		self
	}

	fn pinned(&self) -> Option<SmolStr> {
		self.endpoint
			.read()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner)
			.clone()
	}

	/// The PDS this repo lives on, resolved from the did on first use.
	pub async fn endpoint(&self) -> Result<SmolStr> {
		if let Some(endpoint) = self.pinned() {
			return Ok(endpoint);
		}
		let endpoint =
			self.resolver.resolve(&self.did).await?.pds_endpoint()?;
		*self
			.endpoint
			.write()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner) =
			Some(endpoint.clone());
		endpoint.xok()
	}

	fn auth(&self) -> Result<&Arc<dyn AtprotoAuth>> {
		self.auth.as_ref().ok_or_else(|| {
			bevyhow!(
				"the repo of {} is read only: its `<AtprotoAccount/>` names no \
				 `secret`, the env var holding an app password",
				self.did
			)
		})
	}

	/// Call `method`, a 2xx answer or its [`XrpcError`]. An `authorized`
	/// call that fails is retried once if the credential recovers.
	async fn call(
		&self,
		method: &str,
		authorized: bool,
		build: impl Fn(String) -> Result<Request>,
	) -> Result<Response> {
		let url = format!("{}/xrpc/{method}", self.endpoint().await?);
		let mut retried = false;
		loop {
			let mut request = build(url.clone())?;
			if authorized {
				request = self.auth()?.authorize(request).await?;
			}
			let response = request.send().await?;
			if response.status().is_ok() {
				return Ok(response);
			}
			let error = XrpcError::read(method, response).await;
			if authorized && !retried && self.auth()?.recover(&error).await? {
				retried = true;
				continue;
			}
			return Err(error.into());
		}
	}

	async fn get(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> Result<Option<RecordEntry>> {
		match self
			.call("com.atproto.repo.getRecord", false, |url| {
				Request::get(url)
					.with_param("repo", self.did.as_str())
					.with_param("collection", collection.as_str())
					.with_param("rkey", rkey.as_str())
					.xok()
			})
			.await
		{
			Ok(response) => response.json::<RecordEntry>().await?.xmap(Some),
			Err(err) if XrpcError::is(&err, "RecordNotFound") => None,
			Err(err) => return Err(err),
		}
		.xok()
	}

	async fn put(
		&self,
		collection: Nsid,
		rkey: Rkey,
		record: AtprotoValue,
	) -> Result<StrongRef> {
		let body = PutRecord {
			address: RecordAddress {
				repo: &self.did,
				collection: &collection,
				rkey: &rkey,
			},
			record: &record,
		};
		self.call("com.atproto.repo.putRecord", true, |url| {
			Request::post(url).with_json_body(&body)?.xok()
		})
		.await?
		.json::<StrongRef>()
		.await
	}

	async fn delete(&self, collection: Nsid, rkey: Rkey) -> Result {
		let body = RecordAddress {
			repo: &self.did,
			collection: &collection,
			rkey: &rkey,
		};
		self.call("com.atproto.repo.deleteRecord", true, |url| {
			Request::post(url).with_json_body(&body)?.xok()
		})
		.await?;
		Ok(())
	}

	async fn list(&self, collection: Nsid) -> Result<Vec<RecordEntry>> {
		let mut records = Vec::new();
		let mut cursor: Option<SmolStr> = None;
		loop {
			let page = self
				.call("com.atproto.repo.listRecords", false, |url| {
					let request = Request::get(url)
						.with_param("repo", self.did.as_str())
						.with_param("collection", collection.as_str())
						.with_param("limit", Self::PAGE)
						// oldest first, the rkey order the emulator lists in
						.with_param("reverse", "true");
					match &cursor {
						Some(cursor) => request.with_param("cursor", cursor),
						None => request,
					}
					.xok()
				})
				.await?
				.json::<ListRecordsPage>()
				.await?;
			let done = page.records.is_empty() || page.cursor.is_none();
			records.extend(page.records);
			cursor = page.cursor;
			if done {
				return records.xok();
			}
		}
	}

	async fn upload(
		&self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> Result<BlobRef> {
		self.call("com.atproto.repo.uploadBlob", true, |url| {
			Request::post(url)
				.with_body(bytes.clone())
				.with_content_type(mime_type.clone())
				.xok()
		})
		.await?
		.json::<UploadBlobResponse>()
		.await?
		.blob
		.xok()
	}
}

impl PdsProvider for XrpcPds {
	fn box_clone(&self) -> Box<dyn PdsProvider> { Box::new(self.clone()) }

	fn id(&self) -> &'static str { Self::ID }

	fn did(&self) -> &Did { &self.did }

	fn get_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result<Option<RecordEntry>>> {
		let this = self.clone();
		Box::pin(async move { this.get(collection, rkey).await })
	}

	fn put_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
		record: AtprotoValue,
	) -> SendBoxedFuture<Result<StrongRef>> {
		let this = self.clone();
		Box::pin(async move { this.put(collection, rkey, record).await })
	}

	fn delete_record(
		&self,
		collection: Nsid,
		rkey: Rkey,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move { this.delete(collection, rkey).await })
	}

	fn list_records(
		&self,
		collection: Nsid,
	) -> SendBoxedFuture<Result<Vec<RecordEntry>>> {
		let this = self.clone();
		Box::pin(async move { this.list(collection).await })
	}

	fn upload_blob(
		&self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> SendBoxedFuture<Result<BlobRef>> {
		let this = self.clone();
		Box::pin(async move { this.upload(bytes, mime_type).await })
	}
}

/// An xrpc call the server refused: the method, the status, and the
/// protocol's `{error, message}` body.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{method} answered {status} {error}: {message}")]
pub struct XrpcError {
	/// The xrpc method called, ie `com.atproto.repo.putRecord`.
	pub method: SmolStr,
	/// The http status.
	pub status: StatusCode,
	/// The protocol's error name, ie `ExpiredToken`, `RecordNotFound`.
	pub error: SmolStr,
	/// The server's explanation.
	pub message: String,
	/// The server's `DPoP-Nonce`, which an OAuth credential retries with.
	pub dpop_nonce: Option<SmolStr>,
}

impl XrpcError {
	/// Read the refusal `response` carries; a body that is not the
	/// protocol's error shape keeps its text as the message.
	pub async fn read(method: &str, response: Response) -> Self {
		let status = response.status();
		let dpop_nonce = response
			.headers()
			.get_raw("dpop-nonce")
			.and_then(|values| values.first())
			.map(SmolStr::new);
		let text = response.text().await.unwrap_or_default();
		let body = serde_json::from_str::<XrpcErrorBody>(&text).ok();
		Self {
			method: method.into(),
			status,
			error: body
				.as_ref()
				.map(|body| body.error.clone())
				.unwrap_or_default(),
			message: body.and_then(|body| body.message).unwrap_or(text),
			dpop_nonce,
		}
	}

	/// Whether `err` is an xrpc refusal named `name`.
	pub fn is(err: &BevyError, name: &str) -> bool {
		err.downcast_ref::<Self>()
			.is_some_and(|error| error.error == name)
	}
}

#[derive(Deserialize)]
struct XrpcErrorBody {
	#[serde(default)]
	error: SmolStr,
	#[serde(default)]
	message: Option<String>,
}

/// Where a write lands, the `deleteRecord` input.
#[derive(Serialize)]
struct RecordAddress<'a> {
	repo: &'a Did,
	collection: &'a Nsid,
	rkey: &'a Rkey,
}

/// The `putRecord` input.
#[derive(Serialize)]
struct PutRecord<'a> {
	#[serde(flatten)]
	address: RecordAddress<'a>,
	record: &'a AtprotoValue,
}

#[derive(Deserialize)]
struct ListRecordsPage {
	#[serde(default)]
	cursor: Option<SmolStr>,
	records: Vec<RecordEntry>,
}

#[derive(Deserialize)]
struct UploadBlobResponse {
	blob: BlobRef,
}
