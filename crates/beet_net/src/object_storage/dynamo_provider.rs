use crate::prelude::*;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::describe_table::DescribeTableError;
use beet_core::bevybail;
use bevy::prelude::*;
use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;

#[derive(Clone, Deref, DerefMut, Resource)]
pub struct DynamoDbProvider(pub Client);

impl DynamoDbProvider {
	pub async fn create() -> Self {
		Self::create_with_region("us-west-2").await
	}
	pub async fn create_with_region(region: &str) -> Self {
		let region = Region::new(region.to_string());
		let config = aws_config::from_env()
			.region(RegionProviderChain::default_provider().or_else(region))
			.load()
			.await;
		Self(Client::new(&config))
	}
	fn resolve_key(&self, path: &RoutePath) -> String {
		path.to_string().trim_start_matches('/').to_string()
	}
}

impl BucketProvider for DynamoDbProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> {
		Box::new(Self(self.0.clone()))
	}

	fn region(&self) -> Option<String> {
		self.0.config().region().map(|r| r.to_string())
	}

	fn bucket_exists(
		&self,
		table_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		Box::pin(async move {
			match client.describe_table().table_name(&table_name).send().await {
				Ok(out) => {
					// Only report true when the table is ACTIVE; treat DELETING (and anything else) as false
					let status = out.table().and_then(|t| t.table_status());
					Ok(matches!(
						status,
						Some(aws_sdk_dynamodb::types::TableStatus::Active)
					))
				}
				Err(SdkError::ServiceError(service_err))
					if matches!(
						service_err.err(),
						DescribeTableError::ResourceNotFoundException(_)
					) =>
				{
					Ok(false)
				}
				Err(other) => {
					bevybail!("Failed to check table: {:?}", other)
				}
			}
		})
	}

	fn bucket_create(
		&self,
		table_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		Box::pin(async move {
			// Simple table with string PK "id"
			client
				.create_table()
				.table_name(&table_name)
				.attribute_definitions(
					aws_sdk_dynamodb::types::AttributeDefinition::builder()
						.attribute_name("id")
						.attribute_type(
							aws_sdk_dynamodb::types::ScalarAttributeType::S,
						)
						.build()?,
				)
				.key_schema(
					aws_sdk_dynamodb::types::KeySchemaElement::builder()
						.attribute_name("id")
						.key_type(aws_sdk_dynamodb::types::KeyType::Hash)
						.build()?,
				)
				.provisioned_throughput(
					aws_sdk_dynamodb::types::ProvisionedThroughput::builder()
						.read_capacity_units(1)
						.write_capacity_units(1)
						.build()?,
				)
				.send()
				.await?;
			Ok(())
		})
	}

	fn bucket_remove(
		&self,
		table_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		Box::pin(async move {
			client.delete_table().table_name(&table_name).send().await?;
			Ok(())
		})
	}

	fn insert(
		&self,
		table_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		let key = self.resolve_key(path);
		let body = body.to_vec();
		Box::pin(async move {
			use aws_sdk_dynamodb::types::AttributeValue;
			client
				.put_item()
				.table_name(&table_name)
				.item("id", AttributeValue::S(key))
				.item("data", AttributeValue::B(body.into()))
				.send()
				.await?;
			Ok(())
		})
	}

	fn exists(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		let key = self.resolve_key(path);
		Box::pin(async move {
			use aws_sdk_dynamodb::types::AttributeValue;
			let out = client
				.get_item()
				.table_name(&table_name)
				.key("id", AttributeValue::S(key))
				.send()
				.await?;
			Ok(out.item.is_some())
		})
	}

	fn list(
		&self,
		table_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<Vec<RoutePath>>> + Send + 'static>>
	{
		let client = self.0.clone();
		let table_name = table_name.to_string();
		Box::pin(async move {
			let out = client.scan().table_name(&table_name).send().await?;
			let mut paths = Vec::new();
			if let Some(items) = out.items {
				for item in items {
					if let Some(aws_sdk_dynamodb::types::AttributeValue::S(
						id,
					)) = item.get("id")
					{
						paths.push(RoutePath::new(id));
					}
				}
			}
			Ok(paths)
		})
	}

	fn get(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		let key = self.resolve_key(path);
		Box::pin(async move {
			use aws_sdk_dynamodb::types::AttributeValue;
			let out = client
				.get_item()
				.table_name(&table_name)
				.key("id", AttributeValue::S(key))
				.send()
				.await?;
			if let Some(item) = out.item {
				if let Some(AttributeValue::B(data)) = item.get("data") {
					Ok(Bytes::from(data.clone().into_inner()))
				} else {
					bevybail!("No data field found");
				}
			} else {
				bevybail!("Item not found");
			}
		})
	}

	fn remove(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let client = self.0.clone();
		let table_name = table_name.to_string();
		let key = self.resolve_key(path);
		Box::pin(async move {
			use aws_sdk_dynamodb::types::AttributeValue;
			client
				.delete_item()
				.table_name(&table_name)
				.key("id", AttributeValue::S(key))
				.send()
				.await?;
			Ok(())
		})
	}

	fn public_url(
		&self,
		_table_name: &str,
		_path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'static>>
	{
		// DynamoDB does not have a public URL for items
		Box::pin(async move { Ok(None) })
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[tokio::test]
	// #[ignore = "hits remote dynamodb"]
	async fn works() {
		let provider = DynamoDbProvider::create().await;
		bucket_test::run(provider).await;
	}
}
