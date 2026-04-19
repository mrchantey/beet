use crate::prelude::*;
use aws_config::Region;
use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_bucket::HeadBucketError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use beet_core::prelude::*;
use bytes::Bytes;

/// AWS S3 bucket provider storing its configuration as serializable fields.
/// The S3 client is lazily constructed and cached by region using a [`LazyPool`].
#[derive(Debug, Clone, Component, Reflect, Get)]
#[reflect(Component)]
#[component(on_add = Bucket::on_add::<Self>)]
pub struct S3Bucket {
	/// The S3 bucket name.
	bucket_name: SmolStr,
	/// The AWS region for this bucket.
	region: SmolStr,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<RelPath>,
}

#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for S3Bucket {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl S3Bucket {
	/// Create a new S3 bucket for the given bucket name and region.
	pub fn new(
		bucket_name: impl Into<SmolStr>,
		region: impl Into<SmolStr>,
	) -> Self {
		Self {
			bucket_name: bucket_name.into(),
			region: region.into(),
			subdir: None,
		}
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// Construct the full S3 URI including optional subdir.
	pub fn s3_uri(&self) -> String {
		match &self.subdir {
			Some(subdir) => format!("s3://{}/{}/", self.bucket_name, subdir),
			None => format!("s3://{}/", self.bucket_name),
		}
	}

	/// Get or create an S3 client for this bucket's region.
	async fn client(&self) -> Client {
		static POOL: LazyPool<SmolStr, Client, Client> =
			LazyPool::new(|region| {
				Box::pin(async move {
					let region_obj = Region::new(region.to_string());
					let config =
						aws_config::from_env().region(region_obj).load().await;
					Client::new(&config)
				})
			});
		POOL.get(&self.region).await
	}

	/// Resolve the S3 object key from a [`RelPath`].
	fn resolve_key(&self, path: &RelPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		}
	}

	/// Create a [`TypedBlob`] handle for a single object in this bucket.
	pub fn blob(&self, path: RelPath) -> TypedBlob<Self> {
		TypedBlob::new(self.clone(), path)
	}
}

impl BucketProvider for S3Bucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		Box::new(S3Bucket {
			bucket_name: self.bucket_name.clone(),
			region: self.region.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn region(&self) -> Option<String> { Some(self.region.to_string()) }

	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
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

	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			let mut req =
				client.create_bucket().bucket(this.bucket_name.as_str());

			// us-east-1 is S3's default region and rejects an explicit
			// LocationConstraint; all other regions require it.
			if this.region.as_str() != "us-east-1" {
				use aws_sdk_s3::types::CreateBucketConfiguration;
				let bucket_config = CreateBucketConfiguration::builder()
					.location_constraint(this.region.as_str().into())
					.build();
				req = req.create_bucket_configuration(bucket_config);
			}
			req.send().await?;
			().xok()
		})
	}

	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
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

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
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

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
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
					Some(RelPath::new(rel))
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

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		async_ext::pin_tokio(async move {
			let client = this.client().await;
			let get_result = client
				.get_object()
				.bucket(this.bucket_name.as_str())
				.key(&key)
				.send()
				.await?;
			get_result.body.collect().await?.into_bytes().xok()
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
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

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
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
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let region = &self.region;
		let bucket_name = &self.bucket_name;
		let key = self.resolve_key(path);
		let public_url =
			format!("https://{bucket_name}.s3.{region}.amazonaws.com/{key}");
		Box::pin(async move { Some(public_url).xok() })
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn works() {
		let provider = S3Bucket::new("beet-test-bucket", "us-west-2");
		bucket_test::run(provider).await;
	}

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn infra_bucket() {
		let provider = S3Bucket::new("beet-site-bucket-dev", "us-west-2");
		let bucket = Bucket::new(provider);
		bucket.bucket_try_create().await.unwrap();
		bucket.bucket_exists().await.xpect_ok();

		bucket
			.get(&RelPath::new("index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_starts_with("<!DOCTYPE html>");
	}

	#[beet_core::test]
	#[ignore = "hits remote s3"]
	async fn s3_public_url() {
		let provider = S3Bucket::new("beet-test", "us-west-2");
		let test_key = RelPath::from("test-file.txt");
		Bucket::new(provider)
			.public_url(&test_key)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(format!(
				"https://beet-test.s3.us-west-2.amazonaws.com/{test_key}"
			));
	}
}
