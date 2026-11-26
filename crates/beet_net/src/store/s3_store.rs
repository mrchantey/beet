use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// AWS S3-backed store, holding its configuration as serializable fields.
/// Each call is an [`AwsClient`] request (`restXml`), signed as this store's
/// credential else the launch's own, in its region else the launch's else
/// `us-east-1` (S3 names a bucket's own region on a miss, and the client
/// follows it).
#[derive(Debug, Clone, Component, Reflect, Get)]
#[reflect(Component)]
#[component(on_insert = BlobStore::on_insert::<Self>)]
pub struct S3Store {
	/// The S3 bucket name.
	bucket_name: SmolStr,
	/// The region this store's client is pinned to, else the launch's
	/// (`AWS_REGION`). A store built at the process boundary leaves it unset;
	/// one built from a declaration carries the region that declaration
	/// resolved.
	#[reflect(default)]
	region: Option<SmolStr>,
	/// Optional subdirectory prefix for all keys.
	#[reflect(default)]
	subdir: Option<RelPath>,
	/// Optional S3 endpoint override. Unset uses the default AWS endpoint;
	/// set (with path-style addressing) targets an S3-compatible service such as
	/// Cloudflare R2. See [`S3Store::r2`].
	#[reflect(default)]
	endpoint: Option<SmolStr>,
	/// Whether the bucket's objects are anonymously readable at their url, so a
	/// static-file route may redirect a visitor straight to the bucket instead of
	/// streaming the bytes. Buckets are private unless a policy says otherwise,
	/// so this is `false` by default: claiming a url the bucket will not serve
	/// turns every static file into a redirect to a 403.
	#[reflect(default)]
	public: bool,
	/// The storage class every write lands in (`STANDARD_IA`, `GLACIER_IR`),
	/// else the bucket's default.
	#[reflect(default)]
	storage_class: Option<SmolStr>,
	/// Send every request unsigned, a read of a public bucket from a launch
	/// that holds no credential for it.
	#[reflect(default)]
	anonymous: bool,
	/// An explicit credential, else the launch's own. Set for a bucket at
	/// another vendor (an R2 bucket under the token pair an apply parked)
	/// reached from a process whose ambient credentials are AWS's. Never
	/// reflected, and printed as its key id alone.
	#[reflect(ignore)]
	credentials: Option<AwsCredentials>,
}

impl S3Store {
	/// Create a new S3 store for the given bucket name and region.
	pub fn new(
		bucket_name: impl Into<SmolStr>,
		region: impl Into<SmolStr>,
	) -> Self {
		Self::new_default_region(bucket_name).with_region(region)
	}

	/// Create a store whose region the launch names, the process-boundary
	/// convention: a `--repo=s3://<bucket>` names a bucket, not a region.
	///
	/// `pub(crate)` deliberately. A store is below [`Stack`] in the crate graph
	/// (`beet_infra` depends on `beet_net`, never the reverse), so it cannot
	/// resolve an ancestor stack's region and a region-less one spawned in a
	/// tree would silently ignore it. Stack-relative resolution belongs to the
	/// DECLARATION (`<S3BucketBlock/>`), whose attach observer
	/// hands the resolved region in. If a raw store ever does need it, the way
	/// to add it is an `InfraPlugin` observer: on add of an `S3Store` with no
	/// region, watch that entity for `Ready`, then walk up for a `Stack`
	/// ancestor and apply its region. That is deliberately not done today,
	/// because it would make an unset region mean two different things
	/// depending on whether `InfraPlugin` is linked.
	pub(crate) fn new_default_region(bucket_name: impl Into<SmolStr>) -> Self {
		Self {
			bucket_name: bucket_name.into(),
			region: None,
			subdir: None,
			endpoint: None,
			public: false,
			storage_class: None,
			anonymous: false,
			credentials: None,
		}
	}

	/// Pin this store's client to `region`.
	pub fn with_region(mut self, region: impl Into<SmolStr>) -> Self {
		self.region = Some(region.into());
		self
	}

	/// Create a store backed by Cloudflare R2 through its S3-compatible API. The
	/// endpoint is derived from the account id and the region is always `auto`;
	/// the client uses path-style addressing, as R2 requires. The deployed
	/// container reads the site through this exactly as it would from S3.
	pub fn r2(
		account_id: impl AsRef<str>,
		bucket_name: impl Into<SmolStr>,
	) -> Self {
		Self::new(bucket_name, "auto").with_endpoint(format!(
			"https://{}.r2.cloudflarestorage.com",
			account_id.as_ref()
		))
	}

	/// The store an `s3://` [`StoreUri`] names, erroring on any other kind. An
	/// `endpoint` (eg `https://<account>.r2.cloudflarestorage.com`) switches
	/// onto an S3-compatible service with region `auto`, so one binary serves
	/// identically on AWS S3 and R2; otherwise an unnamed region is the
	/// launch's, the process boundary's region convention (the deploy writes `Environment=AWS_REGION=..` into the
	/// unit). A `path_prefix` roots the store inside the bucket, so a binary
	/// baked with `s3://<bucket>/<deploy-id>` reads only the document version
	/// it shipped with.
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		let StoreUri::S3 {
			name,
			path_prefix,
			endpoint,
			region,
		} = uri
		else {
			bevybail!("store `{uri}` is not an s3:// bucket");
		};
		let store = match (endpoint, region) {
			(Some(endpoint), region) => {
				Self::new(name.clone(), region.as_deref().unwrap_or("auto"))
					.with_endpoint(endpoint.clone())
			}
			(None, Some(region)) => Self::new(name.clone(), region.clone()),
			(None, None) => Self::new_default_region(name.clone()),
		};
		match path_prefix {
			Some(path_prefix) => store.with_subdir(path_prefix.clone()),
			None => store,
		}
		.xok()
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// Override the S3 endpoint, switching the client to path-style addressing for
	/// an S3-compatible service (eg Cloudflare R2, MinIO).
	pub fn with_endpoint(mut self, endpoint: impl Into<SmolStr>) -> Self {
		self.endpoint = Some(endpoint.into());
		self
	}

	/// Sign every request as `credentials` rather than the launch's own,
	/// see [`S3Store::credentials`].
	pub fn with_credentials(mut self, credentials: AwsCredentials) -> Self {
		self.credentials = Some(credentials);
		self
	}

	/// Write every object in `storage_class` rather than the bucket's
	/// default, ie `STANDARD_IA` for a cold copy.
	pub fn with_storage_class(
		mut self,
		storage_class: impl Into<SmolStr>,
	) -> Self {
		self.storage_class = Some(storage_class.into());
		self
	}

	/// Send every request unsigned: a public bucket read by a launch holding
	/// no credential for it.
	pub fn with_anonymous(mut self, anonymous: bool) -> Self {
		self.anonymous = anonymous;
		self
	}

	/// Declare the bucket's objects anonymously readable, so
	/// [`BlobStoreProvider::public_url`] hands out its url and a static-file route
	/// redirects to the bucket rather than streaming through the app. Only true
	/// when a bucket policy (or an R2 custom domain) actually grants public read.
	pub fn with_public(mut self, public: bool) -> Self {
		self.public = public;
		self
	}

	/// The MD5 an S3 etag carries: the quoted hex digest of a single-part
	/// object, and nothing for a multipart upload, whose etag is a digest of
	/// digests no local file can reproduce.
	fn etag_md5(etag: Option<&str>) -> Option<SmolStr> {
		etag.map(|etag| etag.trim_matches('"'))
			.filter(|etag| !etag.contains('-'))
			.map(SmolStr::new)
	}

	/// The region requests go to: this store's, else the launch's, else
	/// `us-east-1`, where S3 answers a miss with the bucket's own.
	fn resolved_region(&self) -> SmolStr {
		self.region
			.clone()
			.or_else(AwsClient::launch_region)
			.unwrap_or_else(|| "us-east-1".into())
	}

	/// The client this store calls through: its endpoint, region and
	/// credential.
	fn client(&self) -> AwsClient {
		let client = AwsClient::new(AwsService::S3, self.resolved_region());
		let client = match &self.endpoint {
			Some(endpoint) => client.with_endpoint(endpoint),
			None => client,
		};
		match (self.anonymous, &self.credentials) {
			(true, _) => client.anonymous(),
			(false, Some(credentials)) => {
				client.with_credentials(credentials.clone())
			}
			(false, None) => client,
		}
	}

	/// `request` against this store's bucket.
	async fn send(&self, request: AwsRequest) -> Result<Response> {
		self.client()
			.send(request.with_bucket(self.bucket_name.clone()))
			.await
	}

	/// `request` against this store's bucket, answering its xml.
	async fn send_xml(&self, request: AwsRequest) -> Result<BsxElement> {
		BsxElement::parse_xml(&self.send(request).await?.text().await?)
	}


	/// Every page of `ListObjectsV2` under `prefix`, split at `delimiter`
	/// where one is given.
	async fn list_pages(
		&self,
		prefix: Option<&str>,
		delimiter: Option<&str>,
	) -> Result<Vec<BsxElement>> {
		let mut pages = Vec::new();
		let mut continuation_token: Option<String> = None;
		loop {
			let mut request = AwsRequest::get("ListObjectsV2", "/")
				.with_query("list-type", "2");
			if let Some(prefix) = prefix {
				request = request.with_query("prefix", prefix);
			}
			if let Some(delimiter) = delimiter {
				request = request.with_query("delimiter", delimiter);
			}
			if let Some(token) = continuation_token.take() {
				request = request.with_query("continuation-token", token);
			}
			let page = self.send_xml(request).await?;
			let truncated = page.text_at("IsTruncated") == Some("true");
			continuation_token = page
				.text_at("NextContinuationToken")
				.map(ToString::to_string);
			pages.push(page);
			if !truncated || continuation_token.is_none() {
				break;
			}
		}
		pages.xok()
	}

	/// Every object under this store's prefix with its size and etag, paged
	/// through `ListObjectsV2`.
	async fn list_objects(&self) -> Result<Vec<(RelPath, BlobStat)>> {
		let prefix = self.subdir.as_ref().map(|subdir| format!("{subdir}/"));
		self.list_pages(prefix.as_deref(), None)
			.await?
			.iter()
			.flat_map(|page| page.children_named("Contents"))
			.filter_map(|object| {
				let key = object.text_at("Key")?;
				let rel = match &prefix {
					Some(prefix) => key.strip_prefix(prefix.as_str())?,
					None => key,
				};
				let stat = BlobStat {
					size: object
						.text_at("Size")
						.and_then(|size| size.parse().ok())
						.unwrap_or_default(),
					md5: Self::etag_md5(object.text_at("ETag")),
				};
				Some((RelPath::new(rel), stat))
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Construct the full S3 URI including optional subdir.
	pub fn s3_uri(&self) -> String {
		match &self.subdir {
			Some(subdir) => format!("s3://{}/{}/", self.bucket_name, subdir),
			None => format!("s3://{}/", self.bucket_name),
		}
	}

	/// Turn object versioning on for this store's bucket.
	///
	/// A bucket whose objects are overwritten in place (a tofu state, a
	/// mirrored manifest) has no undo without this, so the caller that CREATES
	/// such a bucket sets it as part of creating it. Only then: a bucket
	/// somebody later turned versioning off on is left the way they left it,
	/// and nothing re-asserts it on every run.
	///
	/// Versions are kept, with no expiry rule: how long an old version is
	/// worth keeping is the bucket's own declaration to make, not this call's.
	pub async fn set_object_versioning(&self) -> Result {
		self.send(
			AwsRequest::put("PutBucketVersioning", "/")
				.with_flag("versioning")
				.with_xml(
					"<VersioningConfiguration \
					xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
					<Status>Enabled</Status></VersioningConfiguration>",
				)
				.xmap(Self::with_content_md5),
		)
		.await?;
		Ok(())
	}

	/// Block every form of public access to this store's bucket, the
	/// account-level guard a bucket that must never serve anonymously sets
	/// when it is created.
	pub async fn block_public_access(&self) -> Result {
		self.send(
			AwsRequest::put("PutPublicAccessBlock", "/")
				.with_flag("publicAccessBlock")
				.with_xml(
					"<PublicAccessBlockConfiguration \
					xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
					<BlockPublicAcls>true</BlockPublicAcls>\
					<IgnorePublicAcls>true</IgnorePublicAcls>\
					<BlockPublicPolicy>true</BlockPublicPolicy>\
					<RestrictPublicBuckets>true</RestrictPublicBuckets>\
					</PublicAccessBlockConfiguration>",
				)
				.xmap(Self::with_content_md5),
		)
		.await?;
		Ok(())
	}

	/// Delete every object under this store's prefix, a thousand per
	/// `DeleteObjects` (the most one takes), answering how many went. A bucket
	/// that does not exist holds nothing, so it answers zero.
	pub async fn clear(&self) -> Result<usize> {
		let prefix = self.subdir.as_ref().map(|subdir| format!("{subdir}/"));
		let pages = match self.list_pages(prefix.as_deref(), None).await {
			Ok(pages) => pages,
			Err(err) if AwsError::is(&err, &["NoSuchBucket"]) => return Ok(0),
			Err(err) => return Err(err),
		};
		let keys = pages
			.iter()
			.flat_map(|page| page.children_named("Contents"))
			.filter_map(|object| object.text_at("Key").map(ToString::to_string))
			.collect::<Vec<_>>();
		for batch in keys.chunks(1000) {
			let objects = batch
				.iter()
				.map(|key| {
					format!(
						"<Object><Key>{}</Key></Object>",
						html_ext::escape_xml_text(key)
					)
				})
				.collect::<String>();
			let deleted = self
				.send_xml(
					AwsRequest::post("DeleteObjects", "/")
						.with_flag("delete")
						.with_xml(format!(
							"<Delete xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
							<Quiet>true</Quiet>{objects}</Delete>"
						))
						.xmap(Self::with_content_md5),
				)
				.await?;
			if let Some(error) = deleted.child("Error") {
				bevybail!(
					"DeleteObjects could not delete `{}`: {} {}",
					error.text_at("Key").unwrap_or_default(),
					error.text_at("Code").unwrap_or_default(),
					error.text_at("Message").unwrap_or_default()
				);
			}
		}
		keys.len().xok()
	}

	/// The region this store's bucket actually lives in, or `None` when there
	/// is no such bucket. `GetBucketLocation` answers from any regional
	/// endpoint, so this is the one call that says where a bucket is without
	/// being told first; S3 reports `us-east-1` as an empty constraint.
	pub async fn bucket_region(&self) -> Result<Option<SmolStr>> {
		match self
			.send_xml(
				AwsRequest::get("GetBucketLocation", "/").with_flag("location"),
			)
			.await
		{
			Ok(location) => match location.text().trim() {
				"" => SmolStr::new("us-east-1"),
				region => SmolStr::new(region),
			}
			.xsome()
			.xok(),
			Err(err) if AwsError::is(&err, &["NoSuchBucket"]) => Ok(None),
			Err(err) => Err(err),
		}
	}

	/// `request` carrying its body's `Content-MD5`, which S3 requires of a
	/// configuration write and a batch delete.
	fn with_content_md5(request: AwsRequest) -> AwsRequest {
		use base64::Engine;
		use md5::Digest;
		let digest = md5::Md5::digest(request.body());
		let encoded = base64::engine::general_purpose::STANDARD.encode(digest);
		request.with_header("content-md5", encoded)
	}

	/// `request` carrying `condition` as its conditional header.
	fn with_condition(
		request: AwsRequest,
		condition: &BlobCondition,
	) -> AwsRequest {
		match condition {
			BlobCondition::Any => request,
			BlobCondition::Absent => request.with_header("if-none-match", "*"),
			BlobCondition::Matches(version) => {
				request.with_header("if-match", version.as_str())
			}
		}
	}

	/// Whether `err` is a failed conditional write: `412`, or `409` for one
	/// racing another.
	fn is_conflict(err: &BevyError) -> bool {
		AwsError::is(err, &["PreconditionFailed", "ConditionalRequestConflict"])
	}

	/// The version a response's `ETag` names.
	fn version_of(response: &Response, key: &str) -> Result<BlobVersion> {
		match response.headers().first_raw("etag") {
			Some(etag) => BlobVersion::new(etag).xok(),
			None => bevybail!("s3 answered `{key}` without an etag"),
		}
	}

	/// Resolve the S3 object key from a [`RelPath`].
	fn resolve_key(&self, path: &RelPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		}
	}

	/// The request path of `key`.
	fn key_path(key: &str) -> String { format!("/{key}") }
}

impl StoreBackend for S3Store {
	fn id(&self) -> &'static str { "s3" }

	fn describe(&self) -> String {
		match &self.region {
			Some(region) => format!("{} ({region})", self.root_key()),
			None => self.root_key().to_string(),
		}
	}

	fn region(&self) -> Option<String> {
		self.region.as_ref().map(ToString::to_string)
	}

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move {
			match this.send(AwsRequest::head("HeadBucket", "/")).await {
				Ok(_) => true.xok(),
				Err(err)
					if AwsError::is(&err, &["NotFound", "NoSuchBucket"]) =>
				{
					false.xok()
				}
				Err(err) => Err(err),
			}
		})
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			// the RESOLVED region, so an unpinned store creates the bucket
			// where its own requests will land. us-east-1 is S3's default
			// region and rejects an explicit LocationConstraint; all other
			// regions (and R2's `auto`) take it or ignore it.
			let region = this.resolved_region();
			let request = AwsRequest::put("CreateBucket", "/");
			let request = match region.as_str() {
				"us-east-1" => request,
				region => request.with_xml(format!(
					"<CreateBucketConfiguration \
					xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
					<LocationConstraint>{}</LocationConstraint>\
					</CreateBucketConfiguration>",
					html_ext::escape_xml_text(region)
				)),
			};
			this.send(request).await?;
			().xok()
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			// only empty buckets can be deleted, so remove all objects first
			S3Store {
				subdir: None,
				..this.clone()
			}
			.clear()
			.await?;
			this.send(AwsRequest::delete("DeleteBucket", "/")).await?;
			().xok()
		})
	}
}

impl BlobStoreProvider for S3Store {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		Box::new(S3Store {
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
			..self.clone()
		})
	}

	fn base(&self) -> Box<dyn BlobStoreProvider> {
		Box::new(S3Store {
			subdir: None,
			..self.clone()
		})
	}

	fn root_key(&self) -> SmolStr { format!("s3:{}", self.bucket_name).into() }

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let request = AwsRequest::put(
			"PutObject",
			Self::key_path(&self.resolve_key(path)),
		)
		.with_body(body);
		let request = match &self.storage_class {
			Some(class) => {
				request.with_header("x-amz-storage-class", class.as_str())
			}
			None => request,
		};
		Box::pin(async move {
			this.send(request).await?;
			().xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let this = self.clone();
		Box::pin(async move {
			this.list_objects()
				.await?
				.into_iter()
				.map(|(path, _)| path)
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// One listing answers every stat: `ListObjectsV2` carries each object's
	/// size and etag, so a mirror of a whole prefix costs one round trip per
	/// page rather than one `HeadObject` per key.
	fn list_stats(&self) -> SendBoxedFuture<Result<Vec<(RelPath, BlobStat)>>> {
		let this = self.clone();
		Box::pin(async move { this.list_objects().await })
	}

	/// `ListObjectsV2` with a `/` delimiter: the common prefixes are the
	/// directories, the contents the files, one page per thousand.
	fn list_dir(&self, path: &RelPath) -> SendBoxedFuture<Result<BlobDir>> {
		let this = self.clone();
		let prefix = match path.as_str().is_empty() {
			true => this.subdir.as_ref().map(|subdir| format!("{subdir}/")),
			false => Some(format!("{}/", this.resolve_key(path))),
		};
		Box::pin(async move {
			let strip = |key: &str| {
				let rest = match &prefix {
					Some(prefix) => key.strip_prefix(prefix.as_str())?,
					None => key,
				};
				rest.trim_end_matches('/').xmap(SmolStr::new).xsome()
			};
			let mut listing = BlobDir::default();
			for page in this.list_pages(prefix.as_deref(), Some("/")).await? {
				listing.dirs.extend(
					page.children_named("CommonPrefixes")
						.filter_map(|common| strip(common.text_at("Prefix")?)),
				);
				listing.files.extend(
					page.children_named("Contents")
						.filter_map(|object| strip(object.text_at("Key")?))
						.filter(|name| !name.is_empty()),
				);
			}
			listing.dedup().xok()
		})
	}

	/// `HeadObject`: the size and etag without the body.
	fn stat(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<BlobStat>>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			match this
				.send(AwsRequest::head("HeadObject", Self::key_path(&key)))
				.await
			{
				Ok(head) => Some(BlobStat {
					size: head
						.headers()
						.first_raw("content-length")
						.and_then(|length| length.parse().ok())
						.unwrap_or_default(),
					md5: Self::etag_md5(head.headers().first_raw("etag")),
				})
				.xok(),
				Err(err) if AwsError::is(&err, &["NotFound", "NoSuchKey"]) => {
					None.xok()
				}
				Err(err) => Err(err),
			}
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			// a missing key is a miss, not a failure: the served 404 is the whole
			// difference between a static host and a broken one.
			match this
				.send(AwsRequest::get("GetObject", Self::key_path(&key)))
				.await
			{
				Ok(response) => response.body.into_bytes().await,
				Err(err) if AwsError::is(&err, &["NoSuchKey", "NotFound"]) => {
					Err(HttpError::new(
						StatusCode::NOT_FOUND,
						format!("object not found: {key}"),
					)
					.into())
				}
				Err(err) => Err(err),
			}
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let stat = self.stat(path);
		Box::pin(async move { stat.await.map(|stat| stat.is_some()) })
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		Box::pin(async move {
			match this.exists(&path).await? {
				true => {
					this.send(AwsRequest::delete(
						"DeleteObject",
						Self::key_path(&key),
					))
					.await?;
					().xok()
				}
				false => {
					bevybail!("Object not found: {}", key)
				}
			}
		})
	}

	fn conditional_writes(&self) -> bool { true }

	/// `GetObject`, the version its `ETag`.
	fn get_versioned(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			match this
				.send(AwsRequest::get("GetObject", Self::key_path(&key)))
				.await
			{
				Ok(response) => {
					let version = Self::version_of(&response, &key)?;
					Some((response.body.into_bytes().await?, version)).xok()
				}
				Err(err) if AwsError::is(&err, &["NoSuchKey", "NotFound"]) => {
					None.xok()
				}
				Err(err) => Err(err),
			}
		})
	}

	/// `PutObject` with `If-None-Match: *` or `If-Match: <etag>`, which S3
	/// and R2 check and write as one step, answering `412` (or `409` for a
	/// write racing another conditional one) when the condition fails.
	fn insert_if(
		&self,
		path: &RelPath,
		body: Bytes,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result<BlobVersion>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		let request =
			AwsRequest::put("PutObject", Self::key_path(&key)).with_body(body);
		let request = match &self.storage_class {
			Some(class) => {
				request.with_header("x-amz-storage-class", class.as_str())
			}
			None => request,
		};
		let request = Self::with_condition(request, &condition);
		Box::pin(async move {
			match this.send(request).await {
				Ok(response) => Self::version_of(&response, &key),
				Err(err) if Self::is_conflict(&err) => {
					Err(BlobConflict::at(&path))
				}
				Err(err) => Err(err),
			}
		})
	}

	/// `DeleteObject` with `If-Match: <etag>` for a version, a `HeadObject`
	/// first for absence; a missing object under [`BlobCondition::Any`] is
	/// already removed.
	fn remove_if(
		&self,
		path: &RelPath,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		Box::pin(async move {
			if condition == BlobCondition::Absent {
				return match this.exists(&path).await? {
					true => Err(BlobConflict::at(&path)),
					false => Ok(()),
				};
			}
			let request = Self::with_condition(
				AwsRequest::delete("DeleteObject", Self::key_path(&key)),
				&condition,
			);
			match this.send(request).await {
				Ok(_) => Ok(()),
				Err(err)
					if Self::is_conflict(&err)
						|| (matches!(condition, BlobCondition::Matches(_))
							&& AwsError::is(&err, &[
								"NoSuchKey",
								"NotFound",
							])) =>
				{
					Err(BlobConflict::at(&path))
				}
				Err(err) => Err(err),
			}
		})
	}

	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		if !self.public {
			// a private bucket has no anonymous url, so the caller streams the
			// bytes rather than redirecting a visitor into a 403.
			return Box::pin(async move { None.xok() });
		}
		let key = self.resolve_key(path);
		let public_url = match &self.endpoint {
			// path-style URL against the override endpoint (eg the R2 S3 API). A
			// truly public URL needs a custom domain or r2.dev binding.
			Some(endpoint) => format!(
				"{}/{}/{key}",
				endpoint.trim_end_matches('/'),
				self.bucket_name
			),
			// virtual-hosted AWS S3 URL, regional when this store names its
			// region, else the global endpoint (which redirects to it).
			None => match &self.region {
				Some(region) => format!(
					"https://{}.s3.{region}.amazonaws.com/{key}",
					self.bucket_name
				),
				None => format!(
					"https://{}.s3.amazonaws.com/{key}",
					self.bucket_name
				),
			},
		};
		Box::pin(async move { Some(public_url).xok() })
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A key pair is never printed, by the store or on its own.
	#[beet_core::test]
	fn credentials_debug_redacts() {
		let store = S3Store::r2("acct", "bucket")
			.with_credentials(AwsCredentials::new("AKIA", "hunter2"));
		format!("{store:?}")
			.xpect_contains("AKIA")
			.xnot()
			.xpect_contains("hunter2");
	}

	/// The store suite against AWS, in the bucket the root entry's `test`
	/// stage provisions (`beet --entry=. test/deploy`), as the `beet` app's
	/// deploy credential.
	#[beet_core::test(timeout_ms = 120_000)]
	#[ignore = "hits remote s3"]
	async fn works() {
		store_test::run_objects(&BlobStore::new(S3Store::new(
			"beet--test--store",
			"us-west-2",
		)))
		.await;
	}

	/// The store suite against a local S3 server (SeaweedFS's gateway on
	/// 8333, its identity `beetlocal`), path style as any endpoint is.
	#[beet_core::test(timeout_ms = 120_000)]
	#[ignore = "needs a local S3 server on 127.0.0.1:8333"]
	async fn local_s3() {
		store_test::run(
			S3Store::new("beet-local-bucket", "us-east-1")
				.with_endpoint("http://127.0.0.1:8333")
				.with_credentials(AwsCredentials::new(
					"beetlocal",
					"beetlocalsecret",
				)),
		)
		.await;
	}

	/// The table suite as json files in a bucket, every write an S3
	/// conditional put, in the bucket the root entry's `test` stage provisions.
	#[beet_core::test(timeout_ms = 600_000)]
	#[ignore = "hits remote s3"]
	async fn table() {
		table_test::run_rows(
			&JsonTableStore::over(BlobStore::new(S3Store::new(
				"beet--test--table",
				"us-west-2",
			)))
			.unwrap(),
		)
		.await;
	}

	/// The table suite against a local S3 server, see [`local_s3`].
	#[beet_core::test(timeout_ms = 120_000)]
	#[ignore = "needs a local S3 server on 127.0.0.1:8333"]
	async fn local_table() {
		table_test::run(
			JsonTableStore::over(BlobStore::new(
				S3Store::new("beet-local-table", "us-east-1")
					.with_endpoint("http://127.0.0.1:8333")
					.with_credentials(AwsCredentials::new(
						"beetlocal",
						"beetlocalsecret",
					)),
			))
			.unwrap(),
		)
		.await;
	}

	#[beet_core::test]
	async fn r2_store_config() {
		// no network: verifies the R2 constructor wiring + path-style public url.
		let store = S3Store::r2("abc123", "my-bucket");
		store.region().as_deref().xpect_eq(Some("auto"));
		store
			.endpoint()
			.as_ref()
			.unwrap()
			.as_str()
			.xpect_eq("https://abc123.r2.cloudflarestorage.com");
		BlobStore::new(store.with_public(true))
			.public_url(&RelPath::from("index.html"))
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(
				"https://abc123.r2.cloudflarestorage.com/my-bucket/index.html",
			);
	}

	/// A private bucket must not hand out a url it will answer with a 403, which
	/// a static-file route would redirect every visitor into.
	#[beet_core::test]
	async fn private_bucket_has_no_public_url() {
		BlobStore::new(S3Store::new("beet--test--store", "us-west-2"))
			.public_url(&RelPath::from("test-file.txt"))
			.await
			.unwrap()
			.xpect_none();
	}

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn s3_public_url() {
		let provider =
			S3Store::new("beet--test--store", "us-west-2").with_public(true);
		let test_key = RelPath::from("test-file.txt");
		BlobStore::new(provider)
			.public_url(&test_key)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(format!(
				"https://beet--test--store.s3.us-west-2.amazonaws.com/{test_key}"
			));
	}
}
