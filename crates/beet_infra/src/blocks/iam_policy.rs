//! The AWS IAM lowering: the [`AccessGrant`]s a stack declared in, an inline
//! policy document out, shared by every AWS compute block.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// An inline IAM policy document under construction: the statements a compute
/// block's runtime identity carries, LOWERED from the [`AccessGrants`] the
/// stack's blocks declared plus the operational statements the compute seeds
/// for itself.
///
/// Provider-agnostic on the declaring side, provider-specific here: a block says
/// "this process reads that bucket" and every AWS compute renders the same IAM
/// statements from it, while a Cloudflare compute renders wrangler bindings from
/// the identical grant. A kind this lowering cannot render FAILS the deploy
/// naming it and the compute that could not lower it: a grant silently dropped
/// is a box that serves until the first request touching that resource, and a
/// FullAccess policy in its place is the same failure inverted, working until it
/// should not.
///
/// Statements render in call order, so a compute places its own relative to the
/// lowered ones. [`read_bucket`](Self::read_bucket) seeds a bucket the compute
/// reads on its own account INTO the lowered read statement, so it is called
/// before [`lower`](Self::lower).
///
/// Every ARN takes its region from the stack, the one place a region is
/// answered, so a grant never carries one of its own. The account segment is a
/// wildcard because the account id is not known at render time; the resource
/// names are already stage-scoped, and a policy cannot grant cross-account
/// access anyway (that needs a resource policy on the far side).
#[derive(Debug, Clone)]
pub struct IamPolicy {
	/// The compute named in the failure when a kind has no lowering, ie
	/// `lightsail instance`.
	compute: SmolStr,
	/// The region every ARN composes with, resolved by the stack.
	region: SmolStr,
	/// Buckets the identity may read, seeded by the compute and appended to by
	/// [`lower`](Self::lower).
	read_buckets: Vec<String>,
	/// The document's statements, in render order.
	statements: Vec<Value>,
}

impl IamPolicy {
	/// A policy for `compute`'s runtime identity, every ARN in `region`.
	pub fn new(
		region: impl Into<SmolStr>,
		compute: impl Into<SmolStr>,
	) -> Self {
		Self {
			compute: compute.into(),
			region: region.into(),
			read_buckets: Vec::new(),
			statements: Vec::new(),
		}
	}

	/// Seed a bucket this compute reads on its own account, ie the artifacts
	/// bucket it pulls its binary from at boot, which nothing declares. Call
	/// before [`lower`](Self::lower), which renders the read statement.
	pub fn read_bucket(mut self, name: impl Into<String>) -> Self {
		self.read_buckets.push(name.into());
		self
	}

	/// Seed a statement of the compute's own, ie its log group.
	pub fn statement(mut self, statement: Value) -> Self {
		self.statements.push(statement);
		self
	}

	/// Lower every declared grant, grouped by kind and scoped to each grant's
	/// own [`AccessPermissions`]. Fails naming any kind this lowering does not
	/// know, which is what replaces the compile-time exhaustiveness a closed
	/// resource enum promised and never delivered (its readers all had
	/// catch-alls).
	pub fn lower(mut self, access: &AccessGrants) -> Result<Self> {
		let compute = self.compute.clone();
		let mut write_buckets = Vec::<String>::new();
		#[cfg(feature = "bindings_aws_dynamo")]
		let mut read_tables = Vec::<String>::new();
		#[cfg(feature = "bindings_aws_dynamo")]
		let mut write_tables = Vec::<String>::new();
		for grant in access.iter() {
			match grant.kind.as_str() {
				S3BucketBlock::ACCESS_KIND => match grant.permissions {
					AccessPermissions::Read => {
						self.read_buckets.push(grant.name.clone())
					}
					AccessPermissions::ReadWrite => {
						write_buckets.push(grant.name.clone())
					}
				},
				#[cfg(feature = "bindings_aws_dynamo")]
				DynamoTableBlock::ACCESS_KIND => match grant.permissions {
					AccessPermissions::Read => {
						read_tables.push(grant.name.clone())
					}
					AccessPermissions::ReadWrite => {
						write_tables.push(grant.name.clone())
					}
				},
				kind => bevybail!(
					"a `{kind}` resource was declared alongside this {compute}, \
					which has no IAM lowering for that kind. Add one to \
					`IamPolicy::lower`, or declare the resource in a stack this \
					compute does not deploy into."
				),
			}
		}

		// every read-only bucket: the deploy publishes them, the process serves
		// them. Absent entirely when nothing is read, rather than a wildcard.
		if !self.read_buckets.is_empty() {
			let resource = Self::bucket_arns(&self.read_buckets);
			self.statements.push(json!({
				"Sid": "ReadStores",
				"Effect": "Allow",
				"Action": ["s3:GetObject", "s3:ListBucket"],
				"Resource": resource
			}));
		}
		if !write_buckets.is_empty() {
			let resource = Self::bucket_arns(&write_buckets);
			self.statements.push(json!({
				"Sid": "WriteStores",
				"Effect": "Allow",
				"Action": [
					"s3:GetObject",
					"s3:ListBucket",
					"s3:PutObject",
					"s3:DeleteObject"
				],
				"Resource": resource
			}));
		}

		// one statement per permission, never one per table: a `Sid` must be
		// unique within an identity policy, so a stack declaring two read/write
		// tables would otherwise render two `DeclaredTables` statements and AWS
		// would reject the whole document as malformed.
		#[cfg(feature = "bindings_aws_dynamo")]
		if !read_tables.is_empty() {
			let resource = self.table_arns(&read_tables);
			self.statements.push(json!({
				"Sid": "ReadTables",
				"Effect": "Allow",
				"Action": Self::READ_TABLE_ACTIONS,
				"Resource": resource
			}));
		}
		#[cfg(feature = "bindings_aws_dynamo")]
		if !write_tables.is_empty() {
			let resource = self.table_arns(&write_tables);
			self.statements.push(json!({
				"Sid": "DeclaredTables",
				"Effect": "Allow",
				"Action": Self::READ_WRITE_TABLE_ACTIONS,
				"Resource": resource
			}));
		}

		self.xok()
	}

	/// Whether this policy would grant nothing at all, ie a stack declaring no
	/// resources for a compute that seeds none of its own. An empty `Statement`
	/// list is not a valid IAM document, so the compute emits no policy resource
	/// rather than an empty one.
	pub fn is_empty(&self) -> bool { self.statements.is_empty() }

	/// The policy document.
	pub fn render(&self) -> String {
		json!({ "Version": "2012-10-17", "Statement": self.statements })
			.to_string()
	}

	/// The dynamodb actions a read-only table grant lowers to.
	const READ_TABLE_ACTIONS: &'static [&'static str] = &[
		"dynamodb:DescribeTable",
		"dynamodb:GetItem",
		"dynamodb:Query",
		"dynamodb:Scan",
		"dynamodb:BatchGetItem",
	];

	/// The dynamodb actions a read/write table grant lowers to.
	const READ_WRITE_TABLE_ACTIONS: &'static [&'static str] = &[
		"dynamodb:DescribeTable",
		"dynamodb:GetItem",
		"dynamodb:PutItem",
		"dynamodb:UpdateItem",
		"dynamodb:DeleteItem",
		"dynamodb:Query",
		"dynamodb:Scan",
		"dynamodb:BatchGetItem",
		"dynamodb:BatchWriteItem",
	];

	/// The bucket-level and object-level arn for each of `buckets`: an s3 action
	/// set spans both, and a statement naming only one of them silently fails
	/// half the calls.
	fn bucket_arns(buckets: &[String]) -> Vec<String> {
		buckets
			.iter()
			.flat_map(|bucket| {
				[
					format!("arn:aws:s3:::{bucket}"),
					format!("arn:aws:s3:::{bucket}/*"),
				]
			})
			.collect()
	}

	/// The arn of each of `tables`, in this policy's region. A table is one arn,
	/// unlike a bucket, since the dynamodb actions all address the table itself.
	#[cfg(feature = "bindings_aws_dynamo")]
	fn table_arns(&self, tables: &[String]) -> Vec<String> {
		let region = &self.region;
		tables
			.iter()
			.map(|table| format!("arn:aws:dynamodb:{region}:*:table/{table}"))
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Statements render in CALL order, so a compute decides where its own sit
	/// relative to the lowered ones, and a seeded bucket joins the lowered read
	/// statement rather than opening a second one.
	#[beet_core::test]
	fn renders_in_call_order() {
		let policy = IamPolicy::new("us-west-2", "test compute")
			.read_bucket("seeded-bucket")
			.lower(&AccessGrants::new(vec![AccessGrant::read(
				S3BucketBlock::ACCESS_KIND,
				"declared-bucket",
			)]))
			.unwrap()
			.statement(json!({ "Sid": "OwnThing" }))
			.render();
		serde_json::from_str::<Value>(&policy).unwrap()["Statement"]
			.as_array()
			.unwrap()
			.len()
			.xpect_eq(2);
		policy
			.as_str()
			.xpect_contains("seeded-bucket")
			.xpect_contains("declared-bucket");
		policy
			.find("OwnThing")
			.unwrap()
			.xpect_greater_than(policy.find("ReadStores").unwrap());
	}

	/// A stack declaring nothing a compute seeds nothing for grants nothing:
	/// an empty document is not valid IAM, so the compute skips the resource.
	#[beet_core::test]
	fn nothing_declared_grants_nothing() {
		IamPolicy::new("us-west-2", "test compute")
			.lower(&default())
			.unwrap()
			.is_empty()
			.xpect_true();
	}

	/// A `Sid` must be unique within an identity policy, so every grant of one
	/// permission collapses into ONE statement. Rendering a statement per table
	/// produced two `DeclaredTables` and AWS rejected the whole document
	/// (`UNIQUE_SIDS_REQUIRED`), which the site hit the moment it declared a
	/// second read/write table.
	#[cfg(feature = "bindings_aws_dynamo")]
	#[beet_core::test]
	fn one_statement_per_permission_keeps_sids_unique() {
		let policy = IamPolicy::new("us-west-2", "test compute")
			.lower(&AccessGrants::new(vec![
				AccessGrant::read_write(
					DynamoTableBlock::ACCESS_KIND,
					"events",
				),
				AccessGrant::read_write(
					DynamoTableBlock::ACCESS_KIND,
					"aggregates",
				),
				AccessGrant::read(DynamoTableBlock::ACCESS_KIND, "reference"),
				AccessGrant::read(S3BucketBlock::ACCESS_KIND, "content"),
				AccessGrant::read_write(S3BucketBlock::ACCESS_KIND, "ops"),
			]))
			.unwrap()
			.render();
		let statements =
			serde_json::from_str::<Value>(&policy).unwrap()["Statement"]
				.as_array()
				.unwrap()
				.clone();
		let sids = statements
			.iter()
			.map(|statement| statement["Sid"].as_str().unwrap().to_string())
			.collect::<Vec<_>>();
		// four permissions declared, four statements, four distinct sids
		sids.len().xpect_eq(4);
		sids.iter().collect::<HashSet<_>>().len().xpect_eq(4);
		// ..and both read/write tables ride the one statement
		policy
			.as_str()
			.xpect_contains("table/events")
			.xpect_contains("table/aggregates")
			.xpect_contains("table/reference");
	}

	/// A kind with no lowering fails the deploy naming both it and the compute
	/// that could not lower it.
	#[beet_core::test]
	fn unknown_kind_names_the_compute() {
		IamPolicy::new("us-west-2", "test compute")
			.lower(&AccessGrants::new(vec![AccessGrant::read(
				"r2_bucket",
				"some-bucket",
			)]))
			.unwrap_err()
			.to_string()
			.xpect_contains("`r2_bucket`")
			.xpect_contains("no IAM lowering")
			.xpect_contains("test compute");
	}
}
