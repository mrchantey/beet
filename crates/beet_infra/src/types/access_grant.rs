//! What a deployed process is allowed to do with the resources declared
//! alongside it, stated by the resource and lowered by the compute.

use beet_core::prelude::*;

/// A permission a running process needs on one declared resource.
///
/// Declared by the resource block (which knows what it is and what a consumer
/// does with it) and **lowered** by the compute block (which knows the
/// provider's mechanism): AWS compute renders IAM statements, Cloudflare compute
/// would render wrangler bindings. Neither side hand-writes the other's ARNs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessGrant {
	pub level: AccessLevel,
	pub resource: AccessResource,
}

impl AccessGrant {
	pub fn read(resource: AccessResource) -> Self {
		Self {
			level: AccessLevel::Read,
			resource,
		}
	}
	pub fn read_write(resource: AccessResource) -> Self {
		Self {
			level: AccessLevel::ReadWrite,
			resource,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
	Read,
	ReadWrite,
}

/// A resource kind plus the resolved name a grant applies to. Extend with a
/// variant per resource kind a block can declare; the compute blocks match on it
/// exhaustively, so a new kind cannot be silently ungranted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessResource {
	S3Bucket { name: String },
	DynamoTable { name: String, region: SmolStr },
}

/// Every grant the blocks of one stack declared, collected once per deploy and
/// handed to the compute blocks that lower them.
#[derive(Debug, Default, Clone, Deref)]
pub struct AccessGrants(Vec<AccessGrant>);

impl AccessGrants {
	pub fn new(grants: Vec<AccessGrant>) -> Self { Self(grants) }

	/// Every bucket name any grant names, deduplicated in declaration order.
	pub fn s3_buckets(&self) -> Vec<&str> {
		self.0
			.iter()
			.filter_map(|grant| match &grant.resource {
				AccessResource::S3Bucket { name } => Some(name.as_str()),
				_ => None,
			})
			.collect::<Vec<_>>()
			.xmap(dedup_preserving_order)
	}

	/// Every read/write table, as `(name, region)` pairs.
	pub fn dynamo_tables(&self) -> Vec<(&str, &str)> {
		self.0
			.iter()
			.filter_map(|grant| match &grant.resource {
				AccessResource::DynamoTable { name, region } => {
					Some((name.as_str(), region.as_str()))
				}
				_ => None,
			})
			.collect::<Vec<_>>()
			.xmap(dedup_preserving_order)
	}
}

/// Declaration order is the deploy's order, so a policy renders identically
/// across runs and a plan shows no spurious diff.
fn dedup_preserving_order<T: PartialEq>(items: Vec<T>) -> Vec<T> {
	let mut out = Vec::<T>::new();
	for item in items {
		if !out.contains(&item) {
			out.push(item);
		}
	}
	out
}
