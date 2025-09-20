use crate::prelude::*;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_bucket::HeadBucketError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;


// pub async fn s3_fs_selector(fs_path: AbsPathBuf) {}


pub fn s3_bucket() -> impl Bundle {
	AsyncAction::new(async move |mut world, entity| {
		let bucket_name = world.resource::<PackageConfig>().html_bucket_name();
		debug!("Connecting to S3 bucket: {bucket_name}");
		let provider = S3Provider::create().await;
		world
			.entity_mut(entity)
			.insert(Bucket::new(provider, bucket_name));
		world
	})
}

#[derive(Clone, Deref, DerefMut, Resource)]
pub struct S3Provider(pub Client);


impl S3Provider {
	/// Create a new S3 client with the default region: `us-west-2`
	pub async fn create() -> Self {
		Self::create_with_region("us-west-2").await
		// let config = aws_config::load_from_env().await;
		// Self(Client::new(&config))
	}
	/// Create a new S3 client with a specific region, ie `us-west-2`
	pub async fn create_with_region(region: &str) -> Self {
		let region = Region::new(region.to_string());
		let config = aws_config::from_env()
			.region(RegionProviderChain::default_provider().or_else(region))
			.load()
			.await;
		Self(Client::new(&config))
	}
	/// s3 buckets dont want a leading slash in the key
	fn resolve_key(&self, path: &RoutePath) -> String {
		path.to_string().trim_start_matches('/').to_string()
	}
}

impl BucketProvider for S3Provider {
	fn box_clone(&self) -> Box<dyn BucketProvider> {
		Box::new(Self(self.0.clone()))
	}

	fn region(&self) -> Option<String> {
		self.0.config().region().map(|r| r.to_string())
	}

	fn bucket_exists(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<bool>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			match client.head_bucket().bucket(&bucket_name).send().await {
				Ok(_) => Ok(true),
				Err(SdkError::ServiceError(service_err))
					if let HeadBucketError::NotFound(_) = service_err.err() =>
				{
					Ok(false)
				}
				Err(other) => {
					bevybail!("Failed to check bucket: {:?}", other)
				}
			}
		})
	}

	fn bucket_create(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			let mut create_bucket_req =
				client.create_bucket().bucket(&bucket_name);

			use aws_sdk_s3::types::CreateBucketConfiguration;

			let mut bucket_config = CreateBucketConfiguration::builder();
			if let Some(region) = client.config().region() {
				bucket_config = bucket_config
					.location_constraint(region.to_string().as_str().into());
			}

			create_bucket_req = create_bucket_req
				.create_bucket_configuration(bucket_config.build());
			create_bucket_req.send().await?;
			Ok(())
		})
	}

	fn bucket_remove(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			// Only empty buckets can be deleted
			// List all objects in the bucket and delete them
			let mut continuation_token = None;
			loop {
				let mut req = client.list_objects_v2().bucket(&bucket_name);
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
									obj.key.map(|k| {
										aws_sdk_s3::types::ObjectIdentifier::builder().key(k).build()
									})
								})
								.collect::<Result<_, _>>()?,
						))
						.build()?;

					client
						.delete_objects()
						.bucket(&bucket_name)
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

			// Now delete the bucket itself
			client.delete_bucket().bucket(&bucket_name).send().await?;
			Ok(())
		})
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result<()>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = self.resolve_key(path);
		Box::pin(async move {
			client
				.put_object()
				.bucket(&bucket_name)
				.key(&key)
				.body(body.to_vec().into())
				.send()
				.await?;
			Ok(())
		})
	}

	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let head_result = client
				.head_object()
				.bucket(&bucket_name)
				.key(&key)
				.send()
				.await;
			match head_result {
				Ok(_) => Ok(true),
				Err(SdkError::ServiceError(service_err))
					if let HeadObjectError::NotFound(_) = service_err.err() =>
				{
					Ok(false)
				}
				Err(err) => Err(err.into()),
			}
		})
	}

	fn list(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			let mut paths = Vec::new();
			let mut continuation_token = None;

			loop {
				let mut req = client.list_objects_v2().bucket(&bucket_name);
				if let Some(token) = &continuation_token {
					req = req.continuation_token(token);
				}
				let list_result = req.send().await?;
				let contents = list_result.contents.unwrap_or_default();
				paths.extend(
					contents
						.into_iter()
						.filter_map(|obj| obj.key.map(RoutePath::new)),
				);

				if list_result.is_truncated == Some(true) {
					continuation_token = list_result.next_continuation_token;
					if continuation_token.is_none() {
						break;
					}
				} else {
					break;
				}
			}

			Ok(paths)
		})
	}

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = self.resolve_key(path);
		// println!("Getting object from bucket: {}, key: {}", bucket_name, key);
		Box::pin(async move {
			let get_result = client
				.get_object()
				.bucket(&bucket_name)
				.key(&key)
				.send()
				.await?;
			// println!("Object retrieved successfully.");
			let body_bytes = get_result.body.collect().await?.into_bytes();
			Ok(body_bytes)
		})
	}

	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<()>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		let path = path.clone();
		let key = this.resolve_key(&path);

		Box::pin(async move {
			match this.exists(&bucket_name, &path).await? {
				true => {
					this.0
						.delete_object()
						.bucket(&bucket_name)
						.key(&key)
						.send()
						.await?;
					Ok(())
				}
				false => {
					bevybail!("Object not found: {}", key);
				}
			}
		})
	}

	fn public_url(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let region = self.region().unwrap_or_else(|| "us-west-2".to_string());
		let bucket_name = bucket_name.to_string();
		let key = self.resolve_key(path);
		let public_url =
			format!("https://{bucket_name}.s3.{region}.amazonaws.com/{key}",);
		Box::pin(async move { Ok(Some(public_url)) })
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[sweet::test]
	#[ignore = "hits remote s3"]
	async fn works() {
		let provider = S3Provider::create().await;
		bucket_test::run(provider).await;
	}

	#[sweet::test]
	#[ignore = "hits remote s3"]
	async fn infra_bucket() -> Result<()> {
		let client = S3Provider::create().await;

		let bucket = Bucket::new(client, "beet-site-bucket-dev".to_string());
		bucket.bucket_try_create().await?;
		bucket.bucket_exists().await.xpect_ok();

		// READ - Download and verify the file
		bucket
			.get(&RoutePath::new("index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_starts_with("<!DOCTYPE html>");
		Ok(())
	}

	#[sweet::test]
	#[ignore = "hits remote s3"]
	async fn s3_public_url() -> Result<()> {
		let bucket_name: &str = "beet-test";

		let client = S3Provider::create().await;
		let test_key = RoutePath::from("test-file.txt");
		Bucket::new(client, bucket_name.to_string())
			.public_url(&test_key)
			.await?
			.unwrap()
			.xpect_eq(format!(
				"https://{bucket_name}.s3.us-west-2.amazonaws.com{test_key}"
			));

		Ok(())
	}
}
