use crate::prelude::*;
use aws_config::Region;
use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::head_bucket::HeadBucketError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use beet_core::prelude::*;
use bytes::Bytes;

/// AWS S3-backed store, holding its configuration as serializable fields.
/// The S3 client is lazily constructed and cached by region using a [`LazyPool`].
#[derive(Debug, Clone, Component, Reflect, Get)]
#[reflect(Component)]
#[component(on_add = BlobStore::on_add::<Self>)]
pub struct S3Store {
	/// The S3 bucket name.
	bucket_name: SmolStr,
	/// The region this store's client is pinned to, else the SDK's own default
	/// provider chain (`AWS_REGION`, the profile, the instance metadata). A
	/// store built at the process boundary leaves it unset; one built from a
	/// declaration carries the region that declaration resolved.
	region: Option<SmolStr>,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<SmolPath>,
	/// Optional S3 endpoint override. Unset uses the default AWS endpoint;
	/// set (with path-style addressing) targets an S3-compatible service such as
	/// Cloudflare R2. See [`S3Store::r2`].
	endpoint: Option<SmolStr>,
	/// Whether the bucket's objects are anonymously readable at their url, so a
	/// static-file route may redirect a visitor straight to the bucket instead of
	/// streaming the bytes. Buckets are private unless a policy says otherwise,
	/// so this is `false` by default: claiming a url the bucket will not serve
	/// turns every static file into a redirect to a 403.
	public: bool,
}

impl S3Store {
	/// Create a new S3 store for the given bucket name and region.
	pub fn new(
		bucket_name: impl Into<SmolStr>,
		region: impl Into<SmolStr>,
	) -> Self {
		Self::new_default_region(bucket_name).with_region(region)
	}

	/// Create a store whose region the SDK's default provider chain resolves,
	/// the process-boundary convention: a `--store=s3://<bucket>` names a
	/// bucket, not a region.
	pub fn new_default_region(bucket_name: impl Into<SmolStr>) -> Self {
		Self {
			bucket_name: bucket_name.into(),
			region: None,
			subdir: None,
			endpoint: None,
			public: false,
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

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<SmolPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// Override the S3 endpoint, switching the client to path-style addressing for
	/// an S3-compatible service (eg Cloudflare R2, MinIO).
	pub fn with_endpoint(mut self, endpoint: impl Into<SmolStr>) -> Self {
		self.endpoint = Some(endpoint.into());
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

	/// Construct the full S3 URI including optional subdir.
	pub fn s3_uri(&self) -> String {
		match &self.subdir {
			Some(subdir) => format!("s3://{}/{}/", self.bucket_name, subdir),
			None => format!("s3://{}/", self.bucket_name),
		}
	}

	/// Get or create an S3 client for this store's region (and endpoint, if set).
	/// Cached by `(region, endpoint)` so an R2 store and an AWS store in the same
	/// region get distinct clients.
	async fn client(&self) -> Client {
		static POOL: LazyPool<
			(Option<SmolStr>, Option<SmolStr>),
			Client,
			Client,
		> = LazyPool::new(|key| {
			let (region, endpoint) = (key.0.clone(), key.1.clone());
			Box::pin(async move {
				// a configured region wins; an unset one leaves the SDK's own
				// default chain in place.
				let mut loader = aws_config::from_env();
				if let Some(region) = region {
					loader = loader.region(Region::new(region.to_string()));
				}
				let config = loader.load().await;
				match endpoint {
					// R2 / S3-compatible: override the endpoint and use
					// path-style addressing, which those services require.
					Some(endpoint) => Client::from_conf(
						aws_sdk_s3::config::Builder::from(&config)
							.endpoint_url(endpoint.to_string())
							.force_path_style(true)
							.build(),
					),
					// the unchanged default AWS path.
					None => Client::new(&config),
				}
			})
		});
		POOL.get(&(self.region.clone(), self.endpoint.clone()))
			.await
	}

	/// Resolve the S3 object key from a [`SmolPath`].
	fn resolve_key(&self, path: &SmolPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		}
	}

	/// Create a [`TypedBlob`] handle for a single object in this store.
	pub fn blob(&self, path: SmolPath) -> TypedBlob<Self> {
		TypedBlob::new(self.clone(), path)
	}
}

impl BlobStoreProvider for S3Store {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		Box::new(S3Store {
			bucket_name: self.bucket_name.clone(),
			region: self.region.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
			endpoint: self.endpoint.clone(),
			public: self.public,
		})
	}

	fn id(&self) -> &'static str { "s3" }

	fn root_key(&self) -> SmolStr { format!("s3:{}", self.bucket_name).into() }

	fn region(&self) -> Option<String> {
		self.region.as_ref().map(ToString::to_string)
	}

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			match client
				.head_bucket()
				.bucket(this.bucket_name.as_str())
				.send()
				.await
			{
				Ok(_) => true.xok(),
				Err(SdkError::ServiceError(service_err))
					if let HeadBucketError::NotFound(_) = service_err.err() =>
				{
					false.xok()
				}
				Err(other) => {
					bevybail!("Failed to check bucket: {:?}", other)
				}
			}
		})
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			let mut req =
				client.create_bucket().bucket(this.bucket_name.as_str());

			// the client's RESOLVED region, so an unpinned store creates the
			// bucket where its own requests will land. us-east-1 is S3's default
			// region and rejects an explicit LocationConstraint; all other
			// regions require it.
			let region = client.config().region().map(ToString::to_string);
			if let Some(region) = region.filter(|region| region != "us-east-1")
			{
				use aws_sdk_s3::types::CreateBucketConfiguration;
				let bucket_config = CreateBucketConfiguration::builder()
					.location_constraint(region.as_str().into())
					.build();
				req = req.create_bucket_configuration(bucket_config);
			}
			req.send().await?;
			().xok()
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			let bucket_name = this.bucket_name.as_str();

			// Only empty buckets can be deleted, so remove all objects first
			let mut continuation_token = None;
			loop {
				let mut req = client.list_objects_v2().bucket(bucket_name);
				if let Some(token) = &continuation_token {
					req = req.continuation_token(token);
				}
				let list_result = req.send().await?;
				let contents = list_result.contents.unwrap_or_default();

				if !contents.is_empty() {
					let delete_objects = aws_sdk_s3::types::Delete::builder()
						.set_objects(Some(
							contents
								.into_iter()
								.filter_map(|obj| {
									obj.key.map(|key| {
										aws_sdk_s3::types::ObjectIdentifier::builder()
												.key(key)
												.build()
									})
								})
								.collect::<Result<_, _>>()?,
						))
						.build()?;

					client
						.delete_objects()
						.bucket(bucket_name)
						.delete(delete_objects)
						.send()
						.await?;
				}

				if list_result.is_truncated == Some(true) {
					continuation_token = list_result.next_continuation_token;
					if continuation_token.is_none() {
						break;
					}
				} else {
					break;
				}
			}

			client.delete_bucket().bucket(bucket_name).send().await?;
			().xok()
		})
	}

	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			client
				.put_object()
				.bucket(this.bucket_name.as_str())
				.key(&key)
				.body(body.to_vec().into())
				.send()
				.await?;
			().xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		let this = self.clone();
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			let bucket_name = this.bucket_name.as_str();
			let prefix = this.subdir.as_ref().map(|s| format!("{}/", s));
			let mut paths = Vec::new();
			let mut continuation_token = None;

			loop {
				let mut req = client.list_objects_v2().bucket(bucket_name);
				if let Some(ref prefix) = prefix {
					req = req.prefix(prefix);
				}
				if let Some(token) = &continuation_token {
					req = req.continuation_token(token);
				}
				let list_result = req.send().await?;
				let contents = list_result.contents.unwrap_or_default();
				paths.extend(contents.into_iter().filter_map(|obj| {
					let key = obj.key?;
					let rel = match &prefix {
						Some(p) => key.strip_prefix(p.as_str())?,
						None => &key,
					};
					Some(SmolPath::new(rel))
				}));

				if list_result.is_truncated == Some(true) {
					continuation_token = list_result.next_continuation_token;
					if continuation_token.is_none() {
						break;
					}
				} else {
					break;
				}
			}

			paths.xok()
		})
	}

	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			// a missing key is a miss, not a failure: the served 404 is the whole
			// difference between a static host and a broken one.
			let get_result = match client
				.get_object()
				.bucket(this.bucket_name.as_str())
				.key(&key)
				.send()
				.await
			{
				Ok(get_result) => get_result,
				Err(SdkError::ServiceError(service_err))
					if let GetObjectError::NoSuchKey(_) = service_err.err() =>
				{
					return Err(HttpError::new(
						StatusCode::NOT_FOUND,
						format!("object not found: {key}"),
					)
					.into());
				}
				Err(err) => return Err(err.into()),
			};
			get_result.body.collect().await?.into_bytes().xok()
		})
	}

	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			match client
				.head_object()
				.bucket(this.bucket_name.as_str())
				.key(&key)
				.send()
				.await
			{
				Ok(_) => true.xok(),
				Err(SdkError::ServiceError(service_err))
					if let HeadObjectError::NotFound(_) = service_err.err() =>
				{
					false.xok()
				}
				Err(err) => Err(err.into()),
			}
		})
	}

	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		async_ext::pin_tokio(async move {
			match this.exists(&path).await? {
				true => {
					let client = this.client().await;
					client
						.delete_object()
						.bucket(this.bucket_name.as_str())
						.key(&key)
						.send()
						.await?;
					().xok()
				}
				false => {
					bevybail!("Object not found: {}", key)
				}
			}
		})
	}

	fn public_url(
		&self,
		path: &SmolPath,
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

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn works() {
		let provider = S3Store::new("beet-test-bucket", "us-west-2");
		store_test::run(provider).await;
	}

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn infra_store() {
		let provider = S3Store::new("beet-site-bucket-dev", "us-west-2");
		let store = BlobStore::new(provider);
		store.store_try_create().await.unwrap();
		store.store_exists().await.xpect_ok();

		store
			.get(&SmolPath::new("index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_starts_with("<!DOCTYPE html>");
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
			.public_url(&SmolPath::from("index.html"))
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
		BlobStore::new(S3Store::new("beet-test", "us-west-2"))
			.public_url(&SmolPath::from("test-file.txt"))
			.await
			.unwrap()
			.xpect_none();
	}

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn s3_public_url() {
		let provider = S3Store::new("beet-test", "us-west-2").with_public(true);
		let test_key = SmolPath::from("test-file.txt");
		BlobStore::new(provider)
			.public_url(&test_key)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(format!(
				"https://beet-test.s3.us-west-2.amazonaws.com/{test_key}"
			));
	}
}
