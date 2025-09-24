use crate::prelude::*;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::create_table::CreateTableError;
use aws_sdk_dynamodb::operation::describe_table::DescribeTableError;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::types::TableStatus;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;

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
	/// remove leading slash for dynamo friendly path
	fn resolve_key(&self, path: &RoutePath) -> AttributeValue {
		let str = path.to_string().trim_start_matches('/').to_string();
		AttributeValue::S(str)
	}

	/// Get the table status, returning None if the table does not exist.
	async fn table_status(
		&self,
		table_name: &str,
	) -> Result<Option<TableStatus>> {
		match self.describe_table().table_name(table_name).send().await {
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
					DescribeTableError::ResourceNotFoundException(_)
				) =>
			{
				Ok(None)
			}
			Err(other) => {
				bevybail!("Failed to check table: {other:?}")
			}
		}
	}

	async fn await_table_create(&self, table_name: &str) -> Result<()> {
		let mut stream = Backoff::default().with_max_attempts(20).stream();
		while let Some(_) = stream.next().await {
			match self.table_status(table_name).await? {
				Some(TableStatus::Creating) => {}
				Some(TableStatus::Active) => return Ok(()),
				status => {
					bevybail!("Unexpected table state: {:?}", status);
				}
			}
		}
		bevybail!("Table did not become active in time");
	}
	async fn await_table_remove(&self, table_name: &str) -> Result<()> {
		let mut stream = Backoff::default().with_max_attempts(20).stream();
		while let Some(_) = stream.next().await {
			match self.table_status(table_name).await? {
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
	fn box_clone(&self) -> Box<dyn BucketProvider> {
		Box::new(Self(self.0.clone()))
	}

	fn region(&self) -> Option<String> {
		self.0.config().region().map(|r| r.to_string())
	}

	fn bucket_exists(&self, table_name: &str) -> SendBoxedFuture<Result<bool>> {
		let table_name = table_name.to_string();
		let this = self.clone();
		Box::pin(async move {
			match this.table_status(&table_name).await {
				Ok(Some(TableStatus::Active)) => Ok(true),
				Ok(Some(_)) => Ok(false),
				Ok(None) => Ok(false),
				Err(err) => Err(err),
			}
		})
	}

	fn bucket_create(&self, table_name: &str) -> SendBoxedFuture<Result<()>> {
		let builder = self.create_table().table_name(table_name);
		let table_name = table_name.to_string();
		let this = self.clone();
		Box::pin(async move {
			// Simple table with string PK "id"
			let result = builder
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
					this.await_table_create(&table_name).await?;
					Ok(())
				}
				Err(SdkError::ServiceError(service_err))
					if let CreateTableError::ResourceInUseException(_) =
						service_err.err() =>
				{
					// already exists
					Ok(())
				}
				Err(err) => bevybail!("Failed to create table: {:?}", err),
			}
		})
	}

	fn bucket_remove(&self, table_name: &str) -> SendBoxedFuture<Result<()>> {
		let fut = self.delete_table().table_name(table_name).send();
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn insert(
		&self,
		table_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result<()>> {
		let fut = self
			.put_item()
			.table_name(table_name)
			.item("id", self.resolve_key(path))
			.item("data", AttributeValue::B(body.to_vec().into()))
			.send();
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn exists(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>> {
		let fut = self
			.get_item()
			.table_name(table_name)
			.key("id", self.resolve_key(path))
			.send();
		Box::pin(async move {
			use aws_sdk_dynamodb::error::SdkError;
			use aws_sdk_dynamodb::operation::get_item::GetItemError;

			match fut.await {
				Ok(out) => Ok(out.item.is_some()),
				Err(SdkError::ServiceError(service_err))
					if matches!(
						service_err.err(),
						GetItemError::ResourceNotFoundException(_)
					) =>
				{
					Ok(false)
				}
				Err(other) => Err(other.into()),
			}
		})
	}

	fn list(
		&self,
		table_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let fut = self.scan().table_name(table_name).send();
		Box::pin(async move {
			let out = fut.await?;
			let mut paths = Vec::new();
			if let Some(items) = out.items {
				for item in items {
					if let Some(AttributeValue::S(id)) = item.get("id") {
						paths.push(RoutePath::new(id));
					}
				}
			}
			Ok(paths)
		})
	}

	/// using the default bucket get, we assume two fields:
	/// - id: the path to the item
	/// - data: the binary data of the item
	/// For typed tables, see [`TableProvider`]
	fn get(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>> {
		let fut = self
			.get_item()
			.table_name(table_name)
			.key("id", self.resolve_key(path))
			.send();
		Box::pin(async move {
			let out = fut.await?;
			let Some(item) = out.item else {
				bevybail!("Item not found");
			};
			let Some(AttributeValue::B(data)) = item.get("data") else {
				bevybail!("No data field found");
			};
			Ok(Bytes::from(data.clone().into_inner()))
		})
	}

	fn remove(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<()>> {
		let fut = self
			.delete_item()
			.table_name(table_name)
			.key("id", self.resolve_key(path))
			.send();
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn public_url(
		&self,
		_table_name: &str,
		_path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}


impl<T: Table> TableProvider<T> for DynamoDbProvider {
	fn set_typed(
		&self,
		table_name: &str,
		path: &RoutePath,
		body: &T,
	) -> SendBoxedFuture<Result> {
		let Ok(item) = serde_dynamo::to_item(body) else {
			return Box::pin(async move {
				bevybail!("Failed to serialize item for dynamo");
			});
		};
		let fut = self
			.put_item()
			.table_name(table_name)
			.set_item(Some(item))
			.item("id", self.resolve_key(path))
			.send();
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn get_typed(
		&self,
		table_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<T>> {
		let fut = self
			.get_item()
			.table_name(table_name)
			.key("id", self.resolve_key(path))
			.send();
		Box::pin(async move {
			let out = fut.await?;
			let Some(item) = out.item else {
				bevybail!("Item not found");
			};
			let item = serde_dynamo::from_item(item)?;
			Ok(item)
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[tokio::test]
	// #[ignore = "this is a wip"]
	async fn works() {
		let provider = DynamoDbProvider::create().await;
		let bucket = Bucket::new(provider.clone(), "beet_test");
		bucket.bucket_create().await.unwrap();
		let mut stream = Backoff::default().with_max_attempts(20).stream();
		while let Some(next) = stream.next().await {
			println!("Waiting for table create... {next:?}");
			if bucket.bucket_exists().await.unwrap() {
				break;
			}
		}
		let mut stream = Backoff::default().with_max_attempts(20).stream();

		bucket.bucket_remove().await.unwrap();
		while let Some(next) = stream.next().await {
			println!("Waiting for table remove... {next:?}");
			if !bucket.bucket_exists().await.unwrap() {
				break;
			}
		}
		// dynamo tables take time to be active awkward to test
		// bucket_test::run(provider).await;
	}
}
