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

#[derive(Clone, Deref, DerefMut, Resource)]
pub struct S3Provider(pub Client);

impl S3Provider {
	pub async fn create() -> Self {
		let config = aws_config::load_from_env().await;
		Self(Client::new(&config))
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
		key: &str,
		body: Bytes,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = key.to_string();
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
		key: &str,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = key.to_string();
		println!("Getting object from bucket: {}, key: {}", bucket_name, key);
		Box::pin(async move {
			let get_result = client
				.get_object()
				.bucket(&bucket_name)
				.key(&key)
				.send()
				.await?;
			println!("Object retrieved successfully.");
			let body_bytes = get_result.body.collect().await?.into_bytes();
			Ok(body_bytes)
		})
	}

	fn delete(
		&self,
		bucket_name: &str,
		key: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let bucket_name = bucket_name.to_string();
		let key = key.to_string();
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
		key: &str,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>> {
		let region = self.0.config().region().map(|r| r.to_string());
		let bucket_name = bucket_name.to_string();
		let key = key.to_string();
		Box::pin(async move {
			let region_str = region.unwrap_or_else(|| "us-west-2".to_string());
			let public_url = format!(
				"https://{}.s3.{}.amazonaws.com/{}",
				bucket_name, region_str, key
			);
			Ok(public_url)
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	const BUCKET_NAME: &str = "beet-test";
	// const BUCKET_NAME: &str = "beet-test";
	const TEST_KEY: &str = "test-file.txt";
	const TEST_CONTENT: &str = "Hello, beet S3 test!";
	const UPDATED_CONTENT: &str = "Updated beet S3 content!";

	#[tokio::test]
	#[ignore = "expensive"]
	async fn s3_client() {
		let s3_client_resource = S3Provider::create().await;
		let _inner_client = &s3_client_resource.0;
	}

	#[tokio::test]
	#[ignore = "expensive"]
	async fn s3_bucket_crud() -> Result<()> {
		let client = S3Provider::create().await;

		let bucket = Bucket::new(client, BUCKET_NAME.to_string());
		bucket.ensure_exists().await?;

		// Verify bucket exists
		bucket.exists().await.xpect().to_be_ok();

		// CREATE - Upload a test file
		bucket
			.insert(TEST_KEY, TEST_CONTENT)
			.await
			.xpect()
			.to_be_ok();

		// READ - Download and verify the file
		bucket
			.get(TEST_KEY)
			.await
			.unwrap()
			.to_vec()
			.xpect()
			.to_be(TEST_CONTENT.as_bytes().to_vec());

		// UPDATE - Modify the file
		bucket
			.insert(TEST_KEY, UPDATED_CONTENT)
			.await
			.xpect()
			.to_be_ok();

		// Verify update
		bucket
			.get(TEST_KEY)
			.await
			.unwrap()
			.to_vec()
			.xpect()
			.to_be(UPDATED_CONTENT.as_bytes().to_vec());

		// DELETE - Remove the test file
		bucket.delete(TEST_KEY).await.xpect().to_be_ok();

		// Verify deletion
		bucket.get(TEST_KEY).await.xpect().to_be_err();
		Ok(())
	}

	#[tokio::test]
	#[ignore = "expensive"]
	async fn s3_public_url() -> Result<()> {
		let client = S3Provider::create().await;
		Bucket::new(client, BUCKET_NAME.to_string())
			.public_url(TEST_KEY)
			.await?
			.xpect()
			.to_be(format!(
				"https://{BUCKET_NAME}.s3.us-west-2.amazonaws.com/{TEST_KEY}"
			));

		Ok(())
	}
}
