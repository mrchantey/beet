use crate::prelude::*;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::types::TableStatus;
use beet_core::prelude::*;
use bytes::Bytes;

/// AWS DynamoDB provider storing its configuration as serializable fields.
/// The DynamoDB client is lazily constructed and cached by region using a [`LazyPool`].
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct DynamoDbProvider {
	/// The DynamoDB table name (maps to "bucket name" in the storage abstraction).
	table_name: SmolStr,
	/// The AWS region for this table.
	region: SmolStr,
}

impl DynamoDbProvider {
	/// Creates a new provider for the given table name and region.
	pub fn new(
		table_name: impl Into<SmolStr>,
		region: impl Into<SmolStr>,
	) -> Self {
		Self {
			table_name: table_name.into(),
			region: region.into(),
		}
	}

	/// Get or create a DynamoDB client for this provider's region.
	async fn client(&self) -> Client {
		static POOL: LazyPool<SmolStr, Client, Client> =
			LazyPool::new(|region| {
				let region_str = region.to_string();
				Box::pin(async move {
					let region_obj = Region::new(region_str);
					let config = aws_config::from_env()
						.region(
							RegionProviderChain::default_provider()
								.or_else(region_obj),
						)
						.load()
						.await;
					Client::new(&config)
				})
			});
		POOL.get(&self.region).await
	}

	/// Remove leading slash for DynamoDB-friendly path.
	fn resolve_key(&self, path: &RoutePath) -> AttributeValue {
		let str = path.to_string().trim_start_matches('/').to_string();
		AttributeValue::S(str)
	}

	/// Get the table status, returning `None` if the table does not exist.
	async fn table_status(&self) -> Result<Option<TableStatus>> {
		let client = self.client().await;
		match client
			.describe_table()
			.table_name(self.table_name.as_str())
			.send()
			.await
		{
			Ok(out) => {
				let Some(desc) = out.table() else {
					bevybail!("Failed to get table description: {out:?}")
				};
				let Some(status) = desc.table_status() else {
					bevybail!("Failed to get table status: {out:?}")
				};
				Ok(Some(status.clone()))
			}
			Err(SdkError::ServiceError(service_err))
				if matches!(
					service_err.err(),
					operation::describe_table::DescribeTableError::ResourceNotFoundException(_)
				) =>
			{
				Ok(None)
			}
			Err(other) => {
				bevybail!("Failed to check table: {other:?}")
			}
		}
	}

	/// Poll until the table becomes active after creation.
	async fn await_table_create(&self) -> Result<()> {
		let mut stream = Backoff::default().with_max_attempts(20).stream();
		while let Some(_) = stream.next().await {
			match self.table_status().await? {
				Some(TableStatus::Creating) => {}
				Some(TableStatus::Active) => return Ok(()),
				status => {
					bevybail!("Unexpected table state: {:?}", status);
				}
			}
		}
		bevybail!("Table did not become active in time");
	}

	/// Poll until the table is fully deleted.
	async fn await_table_remove(&self) -> Result<()> {
		let mut stream = Backoff::default().with_max_attempts(20).stream();
		while let Some(_) = stream.next().await {
			match self.table_status().await? {
				Some(TableStatus::Deleting) => {}
				None => return Ok(()),
				status => {
					bevybail!("Unexpected table state: {:?}", status);
				}
			}
		}
		bevybail!("Table did not delete in time");
	}
}

impl BucketProvider for DynamoDbProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn region(&self) -> Option<String> { Some(self.region.to_string()) }

	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move {
			match this.table_status().await {
				Ok(Some(TableStatus::Active)) => Ok(true),
				Ok(Some(_)) | Ok(None) => Ok(false),
				Err(err) => Err(err),
			}
		})
	}

	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let client = this.client().await;
			let result = client
				.create_table()
				.table_name(this.table_name.as_str())
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
				.await;

			match result {
				Ok(_) => {
					this.await_table_create().await?;
					Ok(())
				}
				Err(err) => bevybail!("Failed to create table: {:?}", err),
			}
		})
	}

	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let client = this.client().await;
			client
				.delete_table()
				.table_name(this.table_name.as_str())
				.send()
				.await?;
			this.await_table_remove().await?;
			Ok(())
		})
	}

	fn insert(&self, path: &RoutePath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let client = this.client().await;
			client
				.put_item()
				.table_name(this.table_name.as_str())
				.item("id", key)
				.item("data", AttributeValue::B(body.to_vec().into()))
				.send()
				.await?;
			Ok(())
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let this = self.clone();
		Box::pin(async move {
			let client = this.client().await;
			let out = client
				.scan()
				.table_name(this.table_name.as_str())
				.send()
				.await?;
			let mut paths = Vec::new();
			if let Some(items) = out.items {
				for item in items {
					if let Some(AttributeValue::S(id)) = item.get("id") {
						paths.push(RoutePath::new(id));
					}
				}
			}
			paths.xok()
		})
	}

	/// Retrieve an object by path.
	///
	/// Assumes a two-field schema: `id` (path) and `data` (binary).
	/// For typed tables, see [`TableProvider`].
	fn get(&self, path: &RoutePath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let client = this.client().await;
			let out = client
				.get_item()
				.table_name(this.table_name.as_str())
				.key("id", key)
				.send()
				.await?;
			let Some(item) = out.item else {
				bevybail!("Item not found");
			};
			let Some(AttributeValue::B(data)) = item.get("data") else {
				bevybail!("No data field found");
			};
			Bytes::from(data.clone().into_inner()).xok()
		})
	}

	fn exists(&self, path: &RoutePath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let client = this.client().await;
			match client
				.get_item()
				.table_name(this.table_name.as_str())
				.key("id", key)
				.send()
				.await
			{
				Ok(out) => Ok(out.item.is_some()),
				Err(SdkError::ServiceError(service_err))
					if matches!(
						service_err.err(),
						aws_sdk_dynamodb::operation::get_item::GetItemError::ResourceNotFoundException(_)
					) =>
				{
					Ok(false)
				}
				Err(other) => Err(other.into()),
			}
		})
	}

	fn remove(&self, path: &RoutePath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let client = this.client().await;
			let result = client
				.delete_item()
				.table_name(this.table_name.as_str())
				.key("id", key)
				.return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
				.send()
				.await?;
			if result.attributes.is_none() {
				bevybail!("Item not found");
			}
			Ok(())
		})
	}

	fn public_url(
		&self,
		_path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}


#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for DynamoDbProvider {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}

	fn insert_row(&self, body: T) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let Ok(item) = serde_dynamo::to_item(body) else {
			return Box::pin(async move {
				bevybail!("Failed to serialize item for dynamo");
			});
		};
		Box::pin(async move {
			let client = this.client().await;
			client
				.put_item()
				.table_name(this.table_name.as_str())
				.set_item(Some(item))
				.send()
				.await?;
			Ok(())
		})
	}

	fn get_row(&self, id: Uuid) -> SendBoxedFuture<Result<T>> {
		let this = self.clone();
		Box::pin(async move {
			let client = this.client().await;
			let out = client
				.get_item()
				.table_name(this.table_name.as_str())
				.key("id", AttributeValue::S(id.to_string()))
				.send()
				.await?;
			let Some(item) = out.item else {
				bevybail!("Item not found");
			};
			let item: T = serde_dynamo::from_item(item)?;
			item.xok()
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	#[ignore = "takes ages"]
	async fn bucket() {
		let provider = DynamoDbProvider::new("beet-test-table", "us-west-2");
		bucket_test::run(provider).await;
	}
	#[cfg(feature = "json")]
	#[beet_core::test]
	#[ignore = "takes ages"]
	async fn table() {
		let provider = DynamoDbProvider::new("beet-test-table", "us-west-2");
		table_test::run(provider).await;
	}
}
