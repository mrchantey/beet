use crate::prelude::*;
use beet_core::prelude::*;
use core::ops::Bound;
use serde_json::Value as JsonValue;
use serde_json::json;

/// AWS DynamoDB as a table store: one Dynamo table holds every partition's
/// rows in a single-table layout, and no bytes, which a bucket holds. Each call is an [`AwsClient`] request
/// (`awsJson1_0`) signed as the launch's own credential, in the region this
/// store names, else the one the launch does ([`AwsClient::launch_region`]).
///
/// # Layout
///
/// - `pk`, the partition key: the partition id, `@` for the declaration's own
///   (never a partition id, whose alphabet has no `@`).
/// - `sk`, the sort key: `<table>#<key>`, so a table is one key range of
///   its partition, scanned with `Query` in the key's byte order.
/// - `row` the row's canonical json text and `cid` its cid.
/// - `gsi1pk`/`gsi1sk` to `gsi4pk`/`gsi4sk`: generic global secondary index
///   slots. A row's nth declared index writes `<partition>#<table>#<index>`
///   and its [`IndexKey`] text into slot n, so declaring an index never
///   provisions infrastructure; a row type declaring more than
///   [`INDEX_SLOTS`](Self::INDEX_SLOTS) is refused with
///   [`TableError::Limit`].
/// - `ttl`, the expiry an archived row is stamped with.
///
/// A precondition is a `ConditionExpression` (`attribute_not_exists(pk)`,
/// `cid = :cid`), and [`apply`](TableProvider::apply) is one
/// `TransactWriteItems`, refused over its limit of
/// [`MAX_BATCH`](Self::MAX_BATCH) writes. An item is at most 400KB. Index scans read a global secondary index, so they are
/// eventually consistent: a row just written may briefly be missing from
/// one, or found at its previous position.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_insert = TableStore::on_insert::<Self>)]
pub struct DynamoStore {
	/// The DynamoDB table name (maps to "store name" in the storage abstraction).
	table_name: SmolStr,
	/// The region this table's client is pinned to, else the launch's
	/// (`AWS_REGION`). A configured region always wins: a declaration that
	/// named its region must not be silently beaten by the environment the
	/// process happens to carry.
	#[reflect(default)]
	region: Option<SmolStr>,
	/// An endpoint override, ie DynamoDB Local, else AWS's regional host.
	#[reflect(default)]
	endpoint: Option<SmolStr>,
	/// The partition this handle reads and writes, the declaration's own
	/// when `None`.
	#[reflect(ignore)]
	partition: Option<PartitionId>,
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
			endpoint: None,
			partition: None,
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

	/// The store a `dynamo://<table>[?region=..]` uri names,
	/// see [`StoreUri::Dynamo`].
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		let StoreUri::Dynamo { name, region } = uri else {
			bevybail!("store `{uri}` is not a dynamo:// table");
		};
		match region {
			Some(region) => Self::new(name.clone(), region.clone()),
			None => Self::new_default_region(name.clone()),
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

	/// The most writes one `TransactWriteItems` takes.
	pub const MAX_BATCH: usize = 100;
	/// The global secondary index slots every table is created with.
	pub const INDEX_SLOTS: usize = 4;
	/// The attribute an expiring item carries its expiry in, epoch seconds.
	pub const TTL_ATTRIBUTE: &'static str = "ttl";

	/// The `pk` this handle's partition keys to.
	fn pk(&self) -> String {
		self.partition
			.as_ref()
			.map_or_else(|| "@".to_string(), |id| id.to_string())
	}

	/// The `sk` a row of `table` at `key` keys to.
	fn row_sk(table: &TableSpec, key: &TableKey) -> String {
		format!("{}#{key}", table.name)
	}

	/// The item key of `sk` in this partition, as wire json.
	fn item_key(&self, sk: &str) -> JsonValue {
		json!({"pk": {"S": self.pk()}, "sk": {"S": sk}})
	}

	/// The string attribute `name` of a wire item.
	fn string_attr<'a>(item: &'a JsonValue, name: &str) -> Option<&'a str> {
		item[name]["S"].as_str()
	}

	/// Whether a wire item's `ttl` has passed: expired, so gone to every read
	/// here though Dynamo has not deleted it yet.
	fn is_expired(item: &JsonValue) -> bool {
		item[Self::TTL_ATTRIBUTE]["N"]
			.as_str()
			.and_then(|ttl| ttl.parse::<i64>().ok())
			.is_some_and(|ttl| ttl <= Timestamp::now().secs())
	}

	/// The stored row a wire item holds.
	fn stored_row(table: &TableSpec, item: &JsonValue) -> Result<StoredRow> {
		let (Some(sk), Some(row), Some(cid)) = (
			Self::string_attr(item, "sk"),
			Self::string_attr(item, "row"),
			Self::string_attr(item, "cid"),
		) else {
			bevybail!("dynamo item is not a row: {item}");
		};
		let Some(key) = sk.strip_prefix(&format!("{}#", table.name)) else {
			bevybail!("dynamo item `{sk}` is not a row of `{}`", table.name);
		};
		StoredRow::from_parts(
			key.into(),
			Cid::parse(cid)?,
			Row::parse_json(row)?,
		)
		.xok()
	}

	/// The refusal of a row type declaring more indexes than there are
	/// slots.
	fn check_index_slots(&self, table: &TableSpec) -> Result {
		match table.indexes.len() > Self::INDEX_SLOTS {
			true => Err(TableError::Limit {
				backend: self.describe().into(),
				what: format!("indexes on `{}`", table.name).into(),
				limit: Self::INDEX_SLOTS,
				found: table.indexes.len(),
			}
			.into()),
			false => Ok(()),
		}
	}

	/// The wire item a row is written as: its key, row text, cid and an
	/// index slot per declared index.
	fn row_item(
		&self,
		table: &TableSpec,
		row: &StoredRow,
	) -> Result<JsonValue> {
		self.check_index_slots(table)?;
		let mut item = json!({
			"pk": {"S": self.pk()},
			"sk": {"S": Self::row_sk(table, row.key())},
			"row": {"S": serde_json::to_string(&row.row().to_json())?},
			"cid": {"S": row.cid().as_str()},
		});
		for (slot, index) in table.indexes.iter().enumerate() {
			let slot = slot + 1;
			item[format!("gsi{slot}pk")] = json!({
				"S": format!("{}#{}#{}", self.pk(), table.name, index.name)
			});
			item[format!("gsi{slot}sk")] = json!({"S": index.entry(row.key(), row.row())?.to_sortable_text()});
		}
		item.xok()
	}

	/// Set `precondition`'s condition on a request body or transaction item:
	/// a row whose `ttl` has passed counts as absent, as every read here
	/// treats it.
	fn with_condition(
		mut body: JsonValue,
		precondition: &Precondition,
	) -> JsonValue {
		let now = json!({"N": Timestamp::now().secs().to_string()});
		let (expression, values) = match precondition {
			Precondition::Any => return body,
			Precondition::Absent => (
				"attribute_not_exists(pk) OR #ttl <= :now",
				json!({":now": now}),
			),
			Precondition::Matches(cid) => (
				"cid = :cid AND (attribute_not_exists(#ttl) OR #ttl > :now)",
				json!({":cid": {"S": cid.as_str()}, ":now": now}),
			),
		};
		body["ConditionExpression"] = json!(expression);
		body["ExpressionAttributeNames"] = json!({"#ttl": Self::TTL_ATTRIBUTE});
		body["ExpressionAttributeValues"] = values;
		body
	}

	/// The conflict at `key`, naming the row there now.
	async fn conflict(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: &Precondition,
	) -> BevyError {
		match self.get_row(table, key).await {
			Ok(found) => TableError::Conflict {
				table: table.name.clone(),
				key: key.clone(),
				expected: precondition.clone(),
				found: found.map(|row| row.cid().clone()),
			}
			.into(),
			Err(err) => err,
		}
	}

	/// Every item `body` queries, following `LastEvaluatedKey` until `enough`
	/// items are read or there are no more, answering whether more remain.
	async fn query(
		&self,
		mut body: JsonValue,
		enough: Option<usize>,
		keep: impl Fn(&JsonValue) -> bool,
	) -> Result<(Vec<JsonValue>, bool)> {
		let mut items = Vec::new();
		loop {
			if let Some(enough) = enough {
				// a few spare for the bounds filtered client side
				body["Limit"] = json!(enough - items.len() + 2);
			}
			let out = self.call("Query", body.clone()).await?;
			items.extend(
				out["Items"]
					.as_array()
					.into_iter()
					.flatten()
					.filter(|item| keep(item))
					.cloned(),
			);
			let more = matches!(
				&out["LastEvaluatedKey"],
				JsonValue::Object(key) if !key.is_empty()
			);
			if enough.is_some_and(|enough| items.len() >= enough) {
				items.truncate(enough.unwrap());
				return (items, true).xok();
			}
			if !more {
				return (items, false).xok();
			}
			body["ExclusiveStartKey"] = out["LastEvaluatedKey"].clone();
		}
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
}

impl StoreBackend for DynamoStore {
	fn id(&self) -> &'static str { "dynamo" }

	fn describe(&self) -> String {
		let name = match &self.partition {
			Some(partition) => {
				format!("dynamo:{}/{partition}", self.table_name)
			}
			None => format!("dynamo:{}", self.table_name),
		};
		match &self.region {
			Some(region) => format!("{name} ({region})"),
			None => name,
		}
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

	/// The single-table layout: `pk`/`sk`, the four index slots each a
	/// global secondary index projecting every attribute, on-demand billing
	/// (an index under provisioned billing needs its own throughput), and
	/// the expiry attribute enabled.
	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.call("CreateTable", Self::create_table_body()).await?;
			this.await_status("CREATING", Some("ACTIVE")).await?;
			this.call(
				"UpdateTimeToLive",
				json!({"TimeToLiveSpecification": {
					"Enabled": true,
					"AttributeName": Self::TTL_ATTRIBUTE
				}}),
			)
			.await?;
			Ok(())
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.call("DeleteTable", json!({})).await?;
			this.await_status("DELETING", None).await
		})
	}
}

impl DynamoStore {
	/// The `CreateTable` request the single-table layout is.
	fn create_table_body() -> JsonValue {
		let mut attributes = vec![
			json!({"AttributeName": "pk", "AttributeType": "S"}),
			json!({"AttributeName": "sk", "AttributeType": "S"}),
		];
		let mut indexes = Vec::new();
		for slot in 1..=Self::INDEX_SLOTS {
			attributes.push(
				json!({"AttributeName": format!("gsi{slot}pk"), "AttributeType": "S"}),
			);
			attributes.push(
				json!({"AttributeName": format!("gsi{slot}sk"), "AttributeType": "S"}),
			);
			indexes.push(json!({
				"IndexName": format!("gsi{slot}"),
				"KeySchema": [
					{"AttributeName": format!("gsi{slot}pk"), "KeyType": "HASH"},
					{"AttributeName": format!("gsi{slot}sk"), "KeyType": "RANGE"}
				],
				"Projection": {"ProjectionType": "ALL"}
			}));
		}
		json!({
			"AttributeDefinitions": attributes,
			"KeySchema": [
				{"AttributeName": "pk", "KeyType": "HASH"},
				{"AttributeName": "sk", "KeyType": "RANGE"}
			],
			"GlobalSecondaryIndexes": indexes,
			"BillingMode": "PAY_PER_REQUEST"
		})
	}

	/// A key condition over `sk` (or an index's sort key) from inclusive
	/// `low` to exclusive `high`, either open: `BETWEEN` where both are set,
	/// the excluded high itself dropped client side.
	fn sort_condition(
		attribute: &str,
		low: Option<String>,
		high: Option<String>,
		values: &mut JsonValue,
	) -> Option<String> {
		match (low, high) {
			(Some(low), Some(high)) => {
				values[":low"] = json!({"S": low});
				values[":high"] = json!({"S": high});
				Some(format!("{attribute} BETWEEN :low AND :high"))
			}
			(Some(low), None) => {
				values[":low"] = json!({"S": low});
				Some(format!("{attribute} >= :low"))
			}
			(None, Some(high)) => {
				values[":high"] = json!({"S": high});
				Some(format!("{attribute} < :high"))
			}
			(None, None) => None,
		}
	}

	/// One write of a transaction, as its `TransactItems` entry.
	fn transact_item(&self, write: &TableWrite) -> Result<JsonValue> {
		match write {
			TableWrite::Put {
				table,
				row,
				precondition,
			} => json!({"Put": Self::with_condition(json!({
				"TableName": self.table_name.as_str(),
				"Item": self.row_item(table, row)?
			}), precondition)}),
			TableWrite::Remove {
				table,
				key,
				precondition,
			} => json!({"Delete": Self::with_condition(json!({
				"TableName": self.table_name.as_str(),
				"Key": self.item_key(&Self::row_sk(table, key))
			}), precondition)}),
		}
		.xok()
	}

	/// The index of the first write a cancelled transaction's message names
	/// as failing its condition, ie `.. reasons [None, ConditionalCheckFailed]`.
	fn failed_condition(message: &str) -> Option<usize> {
		let reasons = message.rsplit_once('[')?.1.split_once(']')?.0;
		reasons
			.split(',')
			.position(|reason| reason.trim() == "ConditionalCheckFailed")
	}
}

impl TableProvider for DynamoStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider> {
		Box::new(self.clone())
	}

	fn partition(&self, id: &PartitionId) -> Box<dyn TableProvider> {
		Box::new(DynamoStore {
			partition: Some(match &self.partition {
				Some(parent) => parent.join(id),
				None => id.clone(),
			}),
			..self.clone()
		})
	}

	fn supports(&self, capability: Capability) -> bool {
		matches!(capability, Capability::AtomicApply | Capability::Indexes)
	}

	fn get_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
	) -> SendBoxedFuture<Result<Option<StoredRow>>> {
		let this = self.clone();
		let table = table.clone();
		let item_key = self.item_key(&Self::row_sk(&table, key));
		Box::pin(async move {
			let out = this
				.call(
					"GetItem",
					json!({"Key": item_key, "ConsistentRead": true}),
				)
				.await?;
			match out["Item"].is_null() || Self::is_expired(&out["Item"]) {
				true => None,
				false => Some(Self::stored_row(&table, &out["Item"])?),
			}
			.xok()
		})
	}

	fn put_row(
		&self,
		table: &TableSpec,
		row: StoredRow,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move {
			let body = Self::with_condition(
				json!({"Item": this.row_item(&table, &row)?}),
				&precondition,
			);
			match this.call("PutItem", body).await {
				Ok(_) => Ok(()),
				Err(err)
					if AwsError::is(&err, &[
						"ConditionalCheckFailedException",
					]) =>
				{
					Err(this.conflict(&table, row.key(), &precondition).await)
				}
				Err(err) => Err(err),
			}
		})
	}

	fn remove_row(
		&self,
		table: &TableSpec,
		key: &TableKey,
		precondition: Precondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let table = table.clone();
		let key = key.clone();
		Box::pin(async move {
			let body = Self::with_condition(
				json!({"Key": this.item_key(&Self::row_sk(&table, &key))}),
				&precondition,
			);
			match this.call("DeleteItem", body).await {
				Ok(_) => Ok(()),
				Err(err)
					if AwsError::is(&err, &[
						"ConditionalCheckFailedException",
					]) =>
				{
					Err(this.conflict(&table, &key, &precondition).await)
				}
				Err(err) => Err(err),
			}
		})
	}

	/// A `Query` of the partition's `<table>#` range, bounds as a `BETWEEN`
	/// over the sort key and the excluded ends dropped client side.
	fn scan_rows(
		&self,
		table: &TableSpec,
		scan: Scan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move {
			let scan = scan.after_key_cursor();
			let sk = |key: &TableKey| Self::row_sk(&table, key);
			let low = match &scan.start {
				Bound::Included(key) | Bound::Excluded(key) => sk(key),
				Bound::Unbounded => format!("{}#", table.name),
			};
			// `$` follows `#`, so every sort key of the table is below it
			let high = match &scan.end {
				Bound::Included(key) | Bound::Excluded(key) => sk(key),
				Bound::Unbounded => format!("{}$", table.name),
			};
			let mut values = json!({":pk": {"S": this.pk()}});
			values[":low"] = json!({"S": low});
			values[":high"] = json!({"S": high});
			let (items, _) = this
				.query(
					json!({
						"KeyConditionExpression": "pk = :pk AND sk BETWEEN :low AND :high",
						"ExpressionAttributeValues": values,
						"ScanIndexForward": !scan.reverse,
						"ConsistentRead": true
					}),
					scan.limit.map(|limit| limit + 1),
					|item| {
						!Self::is_expired(item)
							&& Self::string_attr(item, "sk")
								.and_then(|sk| {
									sk.strip_prefix(&format!("{}#", table.name))
								})
								.is_some_and(|key| {
									scan.contains(&TableKey::from(key))
								})
					},
				)
				.await?;
			let mut rows = items
				.iter()
				.map(|item| Self::stored_row(&table, item))
				.collect::<Result<Vec<_>>>()?;
			if scan.reverse {
				rows.reverse();
			}
			ScanPage::from_sorted(rows, &scan).xok()
		})
	}

	fn apply(&self, writes: Vec<TableWrite>) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			if writes.len() > Self::MAX_BATCH {
				return Err(TableError::Limit {
					backend: this.describe().into(),
					what: "writes in one batch".into(),
					limit: Self::MAX_BATCH,
					found: writes.len(),
				}
				.into());
			}
			if writes.is_empty() {
				return Ok(());
			}
			let items = writes
				.iter()
				.map(|write| this.transact_item(write))
				.collect::<Result<Vec<_>>>()?;
			let body = json!({"TransactItems": items});
			match this.client()?.call_json("TransactWriteItems", &body).await {
				Ok(_) => Ok(()),
				Err(err) => {
					let failed = AwsError::of(&err)
						.filter(|aws| {
							aws.error_code() == "TransactionCanceledException"
						})
						.and_then(|aws| Self::failed_condition(aws.message()));
					match failed.and_then(|idx| writes.get(idx)) {
						Some(write) => Err(this
							.conflict(
								write.table(),
								write.key(),
								write.precondition(),
							)
							.await),
						None => Err(err),
					}
				}
			}
		})
	}

	/// Stamp each row's `ttl` with now, so Dynamo deletes it for free within
	/// a day or two and every read here skips it from this moment.
	fn expire_rows(
		&self,
		table: &TableSpec,
		keys: Vec<TableKey>,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let table = table.clone();
		Box::pin(async move {
			let now = Timestamp::now().secs();
			keys.iter()
				.map(|key| {
					this.call(
						"UpdateItem",
						json!({
							"Key": this.item_key(&Self::row_sk(&table, key)),
							"UpdateExpression": "SET #ttl = :ttl",
							"ConditionExpression": "attribute_exists(pk)",
							"ExpressionAttributeNames": {"#ttl": Self::TTL_ATTRIBUTE},
							"ExpressionAttributeValues": {":ttl": {"N": now.to_string()}}
						}),
					)
				})
				.xmap(|updates| {
					async_ext::join_all_bounded(BlobStore::GET_ALL_CONCURRENCY, updates)
				})
				.await
				.into_iter()
				.filter(|result| {
					!matches!(result, Err(err) if AwsError::is(err, &["ConditionalCheckFailedException"]))
				})
				.collect::<Result<Vec<_>>>()?;
			Ok(())
		})
	}

	/// A `Query` of the index's slot: its `<partition>#<table>#<index>`
	/// partition and a range over the entries' sortable text.
	fn scan_index(
		&self,
		table: &TableSpec,
		index: &TableIndex,
		scan: IndexScan,
	) -> SendBoxedFuture<Result<ScanPage>> {
		let this = self.clone();
		let table = table.clone();
		let index = index.clone();
		Box::pin(async move {
			this.check_index_slots(&table)?;
			let Some(slot) =
				table.indexes.iter().position(|declared| *declared == index)
			else {
				bevybail!(
					"table `{}` declares no index `{}`",
					table.name,
					index.name
				);
			};
			let slot = slot + 1;
			let range = scan.entry_range()?;
			let mut values = json!({":pk": {"S": format!("{}#{}#{}", this.pk(), table.name, index.name)}});
			let mut condition = format!("gsi{slot}pk = :pk");
			if let Some(sort) = Self::sort_condition(
				&format!("gsi{slot}sk"),
				range.0.as_ref().map(IndexKey::to_sortable_text),
				range.1.as_ref().map(IndexKey::to_sortable_text),
				&mut values,
			) {
				condition.push_str(&format!(" AND {sort}"));
			}
			let sk_attr = format!("gsi{slot}sk");
			let (items, _) = this
				.query(
					json!({
						"IndexName": format!("gsi{slot}"),
						"KeyConditionExpression": condition,
						"ExpressionAttributeValues": values,
						"ScanIndexForward": !scan.reverse
					}),
					scan.limit.map(|limit| limit + 1),
					|item| {
						!Self::is_expired(item)
							&& Self::string_attr(item, &sk_attr)
								.and_then(|text| {
									IndexKey::from_sortable_text(text).ok()
								})
								.is_some_and(|entry| {
									IndexScan::contains(&range, &entry)
								})
					},
				)
				.await?;
			let mut rows = items
				.iter()
				.map(|item| Self::stored_row(&table, item))
				.collect::<Result<Vec<_>>>()?;
			let more = scan.limit.is_some_and(|limit| rows.len() > limit);
			if let Some(limit) = scan.limit {
				rows.truncate(limit);
			}
			let cursor = match (more, rows.last()) {
				(true, Some(last)) => {
					Some(index.entry(last.key(), last.row())?.to_cursor())
				}
				_ => None,
			};
			ScanPage { rows, cursor }.xok()
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The table suite against AWS, in the one table the root entry's `test`
	/// stage provisions (`beet --entry=. test/deploy`), as the `beet` app's
	/// deploy credential.
	#[beet_core::test(timeout_ms = 600_000)]
	#[ignore = "hits remote dynamo"]
	async fn table() {
		table_test::run_rows(&TableStore::new(DynamoStore::new(
			"beet--test--dynamo",
			"us-west-2",
		)))
		.await;
	}

	/// The table suite against DynamoDB Local
	/// (`podman run -p 8000:8000 amazon/dynamodb-local`), which checks every
	/// request's wire shape and accepts any signature.
	#[beet_core::test(timeout_ms = 120_000)]
	#[ignore = "needs DynamoDB Local on 127.0.0.1:8000"]
	async fn local_table() {
		table_test::run(TableStore::new(
			DynamoStore::new("beet-local-table", "us-west-2")
				.with_endpoint("http://127.0.0.1:8000"),
		))
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

	/// A row is keyed by its partition and `<table>#<key>`, its index slots
	/// filled in declaration order.
	#[beet_core::test]
	fn row_item_layout() {
		let store = DynamoStore::new("t", "us-west-2");
		let partition = DynamoStore {
			partition: Some(PartitionId::parse("did:plc:abc").unwrap()),
			..store.clone()
		};
		let row = StoredRow::new(
			"k",
			Row::from_value(value!({ "group": "g", "rank": 1 })).unwrap(),
		)
		.unwrap();
		let item = partition
			.row_item(&TableSpec::of::<table_test::IndexedRow>(), &row)
			.unwrap();
		item["pk"]["S"].as_str().unwrap().xpect_eq("did:plc:abc");
		item["sk"]["S"].as_str().unwrap().xpect_eq("indexed_row#k");
		item["gsi1pk"]["S"]
			.as_str()
			.unwrap()
			.xpect_eq("did:plc:abc#indexed_row#by_group");
		item["gsi2pk"]["S"]
			.as_str()
			.unwrap()
			.xpect_eq("did:plc:abc#indexed_row#by_rank");
		store.pk().xpect_eq("@");
		// five indexes overflow the four slots
		let spec = TableSpec::new("t").with_indexes(
			(0..5).map(|idx| TableIndex::new(format!("i{idx}"), ["f"])),
		);
		TableError::of(&store.row_item(&spec, &row).unwrap_err()).xpect_some();
	}

	/// Markup naming only the table and bucket builds each AWS store, every
	/// optional field defaulted.
	#[beet_core::test]
	fn builds_from_the_name_alone() {
		use bevy::reflect::structs::DynamicStruct;
		let mut table = DynamicStruct::default();
		table.insert("table_name", SmolStr::from("t"));
		DynamoStore::from_reflect(&table)
			.unwrap()
			.describe()
			.xpect_eq("dynamo:t");
		let mut bucket = DynamicStruct::default();
		bucket.insert("bucket_name", SmolStr::from("b"));
		S3Store::from_reflect(&bucket)
			.unwrap()
			.root_key()
			.xpect_eq("s3:b");
	}

	#[beet_core::test]
	fn reads_a_cancelled_transaction() {
		DynamoStore::failed_condition(
			"Transaction cancelled, please refer cancellation reasons for specific reasons [None, ConditionalCheckFailed, None]",
		)
		.xpect_eq(Some(1));
		DynamoStore::failed_condition("Transaction cancelled [None]")
			.xpect_none();
	}
}
