use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use serde_dynamo::AttributeValue;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::HashMap as StdHashMap;

/// AWS DynamoDB provider storing its configuration as serializable fields.
/// Each call is an [`AwsClient`] request (`awsJson1_0`) signed as the
/// launch's own credential, in the region this store names, else the one
/// the launch does ([`AwsClient::launch_region`]).
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_insert = on_insert_dynamo)]
pub struct DynamoStore {
	/// The DynamoDB table name (maps to "store name" in the storage abstraction).
	table_name: SmolStr,
	/// The region this table's client is pinned to, else the launch's
	/// (`AWS_REGION`). A configured region always wins: a declaration that
	/// named its region must not be silently beaten by the environment the
	/// process happens to carry.
	region: Option<SmolStr>,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<RelPath>,
	/// An endpoint override, ie DynamoDB Local, else AWS's regional host.
	endpoint: Option<SmolStr>,
}

impl DynamoStore {
	/// Creates a new provider for the given table name and region.
	pub fn new(
		table_name: impl Into<SmolStr>,
		region: impl Into<SmolStr>,
	) -> Self {
		Self::new_default_region(table_name).with_region(region)
	}

	/// A provider whose region the launch names, the process-boundary
	/// convention for a caller holding a table name and nothing else.
	///
	/// `pub(crate)` for the same reason as
	/// [`S3Store::new_default_region`](crate::prelude::S3Store), which carries
	/// the full note: a region-less store in a tree would ignore its ancestor
	/// `Stack`, so stack-relative resolution stays with the declaration.
	pub(crate) fn new_default_region(table_name: impl Into<SmolStr>) -> Self {
		Self {
			table_name: table_name.into(),
			region: None,
			subdir: None,
			endpoint: None,
		}
	}

	/// Pin this provider's client to `region`.
	pub fn with_region(mut self, region: impl Into<SmolStr>) -> Self {
		self.region = Some(region.into());
		self
	}

	/// Send to `endpoint` rather than AWS, ie DynamoDB Local.
	pub fn with_endpoint(mut self, endpoint: impl Into<SmolStr>) -> Self {
		self.endpoint = Some(endpoint.into());
		self
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// The store a `dynamo://<table>[/<path_prefix>][?region=..]` uri names,
	/// see [`StoreUri::Dynamo`].
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		let StoreUri::Dynamo {
			name,
			path_prefix,
			region,
		} = uri
		else {
			bevybail!("store `{uri}` is not a dynamo:// table");
		};
		let store = match region {
			Some(region) => Self::new(name.clone(), region.clone()),
			None => Self::new_default_region(name.clone()),
		};
		match path_prefix {
			Some(path_prefix) => store.with_subdir(path_prefix.clone()),
			None => store,
		}
		.xok()
	}

	/// The client this provider calls through: its region, else the
	/// launch's.
	fn client(&self) -> Result<AwsClient> {
		let Some(region) =
			self.region.clone().or_else(AwsClient::launch_region)
		else {
			bevybail!(
				"dynamo table `{}` names no region and the launch sets no \
				`AWS_REGION`",
				self.table_name
			);
		};
		let client = AwsClient::new(AwsService::DYNAMODB, region);
		match &self.endpoint {
			Some(endpoint) => client.with_endpoint(endpoint),
			None => client,
		}
		.xok()
	}

	/// Call `action` with `body`, the table named in it.
	async fn call(
		&self,
		action: &str,
		mut body: JsonValue,
	) -> Result<JsonValue> {
		body["TableName"] = json!(self.table_name.as_str());
		self.client()?.call_json(action, &body).await
	}

	/// The `id` attribute `path` keys to: the path under this store's subdir.
	fn qualify(&self, path: &RelPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{sub}/{path}"),
			None => path.to_string(),
		}
	}

	/// The key `path` addresses, as DynamoDB's wire json.
	fn key_of(&self, path: &RelPath) -> JsonValue {
		json!({"id": {"S": self.qualify(path)}})
	}

	/// An item from DynamoDB's wire json.
	fn item_of(json: &JsonValue) -> Result<StdHashMap<String, AttributeValue>> {
		serde_json::from_value::<StdHashMap<String, AttributeValue>>(
			json.clone(),
		)
		.map_err(BevyError::from)
	}

	/// The document a table item carries under `row`.
	fn row_of(mut item: StdHashMap<String, AttributeValue>) -> Result<Value> {
		let Some(row) = item.remove("row") else {
			bevybail!("item carries no row document");
		};
		serde_dynamo::from_attribute_value::<_, Value>(row)?.xok()
	}

	/// Get the table status, returning `None` if the table does not exist.
	async fn table_status(&self) -> Result<Option<SmolStr>> {
		match self.call("DescribeTable", json!({})).await {
			Ok(out) => match out["Table"]["TableStatus"].as_str() {
				Some(status) => Ok(Some(status.into())),
				None => bevybail!("DescribeTable answered no table status"),
			},
			Err(err) if AwsError::is(&err, &["ResourceNotFoundException"]) => {
				Ok(None)
			}
			Err(err) => Err(err),
		}
	}

	/// Poll until the table reaches `target` (`None` for deleted), passing
	/// through `transitional`.
	async fn await_status(
		&self,
		transitional: &str,
		target: Option<&str>,
	) -> Result {
		// the waits up front, so no rng is held across an await and the
		// future stays `Send`
		let waits = Backoff::default()
			.with_max_attempts(20)
			.iter()
			.map(|frame| frame.next_attempt)
			.collect::<Vec<_>>();
		for wait in waits {
			match self.table_status().await? {
				Some(status) if status == transitional => {}
				status if status.as_deref() == target => return Ok(()),
				status => {
					bevybail!("Unexpected table state: {:?}", status);
				}
			}
			AwsClient::sleep(wait.unwrap_or_default()).await;
		}
		bevybail!("Table did not reach {target:?} in time");
	}

	/// Every item of a paginated `Scan` over this table, `extra` naming its
	/// projection or filter.
	async fn scan(
		&self,
		extra: JsonValue,
	) -> Result<Vec<StdHashMap<String, AttributeValue>>> {
		let mut items = Vec::new();
		let mut start_key: Option<JsonValue> = None;
		loop {
			let mut body = extra.clone();
			if let Some(key) = start_key.take() {
				body["ExclusiveStartKey"] = key;
			}
			let out = self.call("Scan", body).await?;
			for item in out["Items"].as_array().into_iter().flatten() {
				items.push(Self::item_of(item)?);
			}
			// an absent (or empty) last evaluated key ends the scan
			match &out["LastEvaluatedKey"] {
				JsonValue::Object(key) if !key.is_empty() => {
					start_key = Some(out["LastEvaluatedKey"].clone());
				}
				_ => break,
			}
		}
		items.xok()
	}
}

/// Insert both erased store currencies: the [`BlobStore`] every provider gets,
/// then the [`TableStore`] wrapping this provider directly, so its native
/// document form wins over the json-over-blobs table the blob hook
/// materializes under `json`.
fn on_insert_dynamo(mut world: DeferredWorld, cx: HookContext) {
	BlobStore::on_insert::<DynamoStore>(world.reborrow(), cx);
	TableStore::on_insert::<DynamoStore>(world, cx);
}

impl BlobStoreProvider for DynamoStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		Box::new(DynamoStore {
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
			..self.clone()
		})
	}

	fn base(&self) -> Box<dyn BlobStoreProvider> {
		Box::new(DynamoStore {
			subdir: None,
			..self.clone()
		})
	}

	fn id(&self) -> &'static str { "dynamo" }

	fn root_key(&self) -> SmolStr {
		format!("dynamo:{}", self.table_name).into()
	}

	fn region(&self) -> Option<String> {
		self.region.as_ref().map(ToString::to_string)
	}

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move {
			Ok(this.table_status().await?.as_deref() == Some("ACTIVE"))
		})
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.call(
				"CreateTable",
				json!({
					"AttributeDefinitions": [
						{"AttributeName": "id", "AttributeType": "S"}
					],
					"KeySchema": [{"AttributeName": "id", "KeyType": "HASH"}],
					"ProvisionedThroughput": {
						"ReadCapacityUnits": 1,
						"WriteCapacityUnits": 1
					}
				}),
			)
			.await?;
			this.await_status("CREATING", Some("ACTIVE")).await
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.call("DeleteTable", json!({})).await?;
			this.await_status("DELETING", None).await
		})
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let id = self.qualify(path);
		let data = serde_json::to_value(AttributeValue::B(body.to_vec()));
		Box::pin(async move {
			let item = json!({"id": {"S": id}, "data": data?});
			this.call("PutItem", json!({"Item": item})).await?;
			Ok(())
		})
	}

	/// List every key in the table.
	///
	/// A DynamoDB `Scan` returns at most 1MB per call, so this follows
	/// `LastEvaluatedKey` to exhaustion; a single call silently truncates a
	/// table of any real size (prod analytics returned 4k of 200k rows) and the
	/// caller cannot tell a truncated page from a small table. Projects to the
	/// `id` attribute so pages carry keys rather than row bodies, which is both
	/// cheaper and far more keys per page.
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let this = self.clone();
		Box::pin(async move {
			let prefix = this.subdir.as_ref().map(|s| format!("{}/", s));
			this.scan(json!({"ProjectionExpression": "id"}))
				.await?
				.into_iter()
				.filter_map(|item| match item.get("id") {
					Some(AttributeValue::S(id)) => match &prefix {
						Some(prefix) => {
							id.strip_prefix(prefix.as_str()).map(RelPath::new)
						}
						None => Some(RelPath::new(id)),
					},
					_ => None,
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// Retrieve an object by path.
	///
	/// Assumes a two-field schema: `id` (path) and `data` (binary).
	/// For typed tables, see [`TableProvider`].
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.key_of(path);
		Box::pin(async move {
			let out = this.call("GetItem", json!({"Key": key})).await?;
			if out["Item"].is_null() {
				bevybail!("Item not found");
			}
			let Some(AttributeValue::B(data)) =
				Self::item_of(&out["Item"])?.remove("data")
			else {
				bevybail!("No data field found");
			};
			Bytes::from(data).xok()
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let key = self.key_of(path);
		Box::pin(async move {
			match this
				.call(
					"GetItem",
					json!({"Key": key, "ProjectionExpression": "id"}),
				)
				.await
			{
				Ok(out) => Ok(!out["Item"].is_null()),
				Err(err)
					if AwsError::is(&err, &["ResourceNotFoundException"]) =>
				{
					Ok(false)
				}
				Err(err) => Err(err),
			}
		})
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.key_of(path);
		Box::pin(async move {
			let out = this
				.call(
					"DeleteItem",
					json!({"Key": key, "ReturnValues": "ALL_OLD"}),
				)
				.await?;
			if out["Attributes"].is_null() {
				bevybail!("Item not found");
			}
			Ok(())
		})
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}

/// Native document form: a row lands as a structured DynamoDB item (queryable
/// attributes), never json bytes, so this backend needs no `json` codec at
/// all. The item is keyed by [`TableKey::path`] in its `id` attribute, under
/// this store's subdir exactly as a blob is, and carries the document under
/// `row`, so a row's own `id` field never collides with the hash key.
impl TableProvider for DynamoStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn insert_row(
		&self,
		table: &str,
		key: &TableKey,
		row: Value,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let id = self.qualify(&key.path(table));
		let row = serde_dynamo::to_attribute_value::<_, AttributeValue>(row)
			.map_err(BevyError::from)
			.and_then(|row| serde_json::to_value(row).map_err(BevyError::from));
		Box::pin(async move {
			let item = json!({"id": {"S": id}, "row": row?});
			this.call("PutItem", json!({"Item": item})).await?;
			Ok(())
		})
	}

	fn get_row(
		&self,
		table: &str,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Value>> {
		let this = self.clone();
		let key = self.key_of(&key.path(table));
		Box::pin(async move {
			let out = this.call("GetItem", json!({"Key": key})).await?;
			if out["Item"].is_null() {
				bevybail!("Item not found");
			}
			Self::row_of(Self::item_of(&out["Item"])?)
		})
	}

	/// Read every row of `table` with a paginated `Scan` filtered to the
	/// table's `id` prefix, deserializing the items the scan already returned.
	///
	/// The [`TableProvider::get_all_rows`] default would list the keys and then
	/// issue one `GetItem` per row, an N+1 that a scan makes unnecessary: the
	/// scan carries the bodies.
	fn get_all_rows(
		&self,
		table: &str,
	) -> SendBoxedFuture<Result<Vec<(TableKey, Result<Value>)>>> {
		let this = self.clone();
		let prefix = format!("{}/", self.qualify(&RelPath::new(table)));
		Box::pin(async move {
			this.scan(json!({
				"FilterExpression": "begins_with(id, :prefix)",
				"ExpressionAttributeValues": {":prefix": {"S": prefix}}
			}))
			.await?
			.into_iter()
			.filter_map(|item| {
				let Some(AttributeValue::S(id)) = item.get("id") else {
					return None;
				};
				let key = TableKey::from(id.strip_prefix(prefix.as_str())?);
				Some((key, Self::row_of(item)))
			})
			.collect::<Vec<_>>()
			.xok()
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	#[ignore = "takes ages"]
	async fn store() {
		let provider = DynamoStore::new("beet-test-table", "us-west-2");
		store_test::run(provider).await;
	}
	#[beet_core::test]
	#[ignore = "takes ages"]
	async fn table() {
		let provider = DynamoStore::new("beet-test-table", "us-west-2");
		table_test::run(provider).await;
	}

	/// The store and table suites against DynamoDB Local
	/// (`podman run -p 8000:8000 amazon/dynamodb-local`), which checks every
	/// request's wire shape and accepts any signature.
	#[beet_core::test]
	#[ignore = "needs DynamoDB Local on 127.0.0.1:8000"]
	async fn local_store() {
		store_test::run(
			DynamoStore::new("beet-local-store", "us-west-2")
				.with_endpoint("http://127.0.0.1:8000"),
		)
		.await;
	}
	#[beet_core::test]
	#[ignore = "needs DynamoDB Local on 127.0.0.1:8000"]
	async fn local_table() {
		table_test::run(
			DynamoStore::new("beet-local-table", "us-west-2")
				.with_endpoint("http://127.0.0.1:8000"),
		)
		.await;
	}

	/// A configured region wins over the environment, and an unconfigured one
	/// takes the launch's.
	///
	/// REGRESSION: the client used to resolve the environment FIRST, so a box
	/// whose unit exported a different `AWS_REGION` silently talked to a
	/// table in another region than the deploy created, while S3
	/// (configured-wins) talked to the right one.
	#[beet_core::test]
	fn configured_region_beats_env() {
		unsafe { env_ext::set_var("AWS_REGION", "us-east-1") }.unwrap();
		DynamoStore::new("beet-site--dev--analytics", "eu-west-1")
			.client()
			.unwrap()
			.region()
			.xpect_eq("eu-west-1");
		DynamoStore::new_default_region("beet-site--dev--analytics")
			.client()
			.unwrap()
			.region()
			.xpect_eq("us-east-1");
	}

	/// The wire json a blob and a row are written as, and the scan that reads
	/// them back, through the transport seam.
	#[beet_core::test]
	fn items_are_wire_json() {
		serde_json::to_value(AttributeValue::B(b"hi".to_vec()))
			.unwrap()
			.xpect_eq(json!({"B": "aGk="}));
		DynamoStore::item_of(&json!({"id": {"S": "a/b"}, "n": {"N": "3"}}))
			.unwrap()
			.get("id")
			.cloned()
			.xpect_eq(Some(AttributeValue::S("a/b".into())));
	}
}
