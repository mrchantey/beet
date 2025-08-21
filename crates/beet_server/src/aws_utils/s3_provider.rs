use crate::prelude::*;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_bucket::HeadBucketError;
use beet_core::bevybail;
use bevy::prelude::*;
use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;

pub fn s3_bucket() -> impl Bundle {
	AsyncAction::new(async move |mut world, entity| {
		let bucket_name = world.resource::<PackageConfig>().default_bucket_name();
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
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
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

	fn create_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
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

	fn delete_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			client.delete_bucket().bucket(&bucket_name).send().await?;
			Ok(())
		})
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
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

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>> {
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

	fn delete(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = self.resolve_key(path);
		Box::pin(async move {
			client
				.delete_object()
				.bucket(&bucket_name)
				.key(&key)
				.send()
				.await?;
			Ok(())
		})
	}

	fn public_url(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'static>> {
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

	const BUCKET_NAME: &str = "beet-test";
	fn test_key() -> RoutePath { RoutePath::from("test-file.txt") }
	const TEST_CONTENT: &str = "Hello, beet S3 test!";
	const UPDATED_CONTENT: &str = "Updated beet S3 content!";

	#[tokio::test]
	#[ignore = "hits remote s3"]
	async fn s3_client() {
		let s3_client_resource = S3Provider::create().await;
		let _inner_client = &s3_client_resource.0;
	}

	#[tokio::test]
	#[ignore = "hits remote s3"]
	async fn infra_bucket() -> Result<()> {
		let client = S3Provider::create().await;

		let bucket = Bucket::new(client, "beet-site-bucket-dev".to_string());
		bucket.ensure_exists().await?;
		bucket.exists().await.xpect().to_be_ok();

		// READ - Download and verify the file
		bucket
			.get(&RoutePath::new("index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect()
			.to_start_with("<!DOCTYPE html>");
		Ok(())
	}
	#[tokio::test]
	#[ignore = "hits remote s3"]
	async fn s3_bucket_crud() -> Result<()> {
		let client = S3Provider::create().await;

		let bucket = Bucket::new(client, BUCKET_NAME.to_string());
		bucket.ensure_exists().await?;

		// Verify bucket exists
		bucket.exists().await.xpect().to_be_ok();

		let test_key = test_key();

		// CREATE - Upload a test file
		bucket
			.insert(&test_key, TEST_CONTENT)
			.await
			.xpect()
			.to_be_ok();

		// READ - Download and verify the file
		bucket
			.get(&test_key)
			.await
			.unwrap()
			.to_vec()
			.xpect()
			.to_be(TEST_CONTENT.as_bytes().to_vec());

		// UPDATE - Modify the file
		bucket
			.insert(&test_key, UPDATED_CONTENT)
			.await
			.xpect()
			.to_be_ok();

		// Verify update
		bucket
			.get(&test_key)
			.await
			.unwrap()
			.to_vec()
			.xpect()
			.to_be(UPDATED_CONTENT.as_bytes().to_vec());

		// DELETE - Remove the test file
		bucket.delete(&test_key).await.xpect().to_be_ok();

		// Verify deletion
		bucket.get(&test_key).await.xpect().to_be_err();
		Ok(())
	}

	#[tokio::test]
	#[ignore = "hits remote s3"]
	async fn s3_public_url() -> Result<()> {
		let client = S3Provider::create().await;
		let test_key = test_key();
		Bucket::new(client, BUCKET_NAME.to_string())
			.public_url(&test_key)
			.await?
			.xpect()
			.to_be(format!(
				"https://{BUCKET_NAME}.s3.us-west-2.amazonaws.com{test_key}"
			));

		Ok(())
	}
}
