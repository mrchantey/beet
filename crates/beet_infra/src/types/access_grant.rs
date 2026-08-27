//! What a deployed process is allowed to do with the resources declared
//! alongside it, stated by the resource and lowered by the compute.

use beet_core::prelude::*;

/// A permission a running process needs on one declared resource.
///
/// Declared by the resource block (which knows what it is and what a consumer
/// does with it) and **lowered** by the compute block (which knows the
/// provider's mechanism): AWS compute renders IAM statements, Cloudflare compute
/// would render wrangler bindings. Neither side hand-writes the other's ARNs.
///
/// Nothing here is provider-shaped: the [`kind`](Self::kind) is a plain string
/// the declaring block owns, so a Cloudflare block mints `"r2_bucket"` without
/// touching this module. The compute that cannot lower a kind fails loudly
/// naming it, which is what replaces the compile-time exhaustiveness a closed
/// enum used to promise and never delivered (its readers all had catch-alls).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessGrant {
	/// The resource kind, a constant owned by the declaring block, ie
	/// [`S3BucketBlock::ACCESS_KIND`](crate::prelude::S3BucketBlock::ACCESS_KIND).
	pub kind: SmolStr,
	/// The resolved resource name, ie `beet-site--prod--analytics`.
	pub name: String,
	/// What the process may do with it.
	pub permissions: AccessPermissions,
}

impl AccessGrant {
	/// A read-only grant on `name`, ie a bucket the deploy publishes and the
	/// process only serves.
	pub fn read(kind: impl Into<SmolStr>, name: impl Into<String>) -> Self {
		Self {
			kind: kind.into(),
			name: name.into(),
			permissions: AccessPermissions::Read,
		}
	}
	/// A read/write grant on `name`, ie a table the process records to.
	pub fn read_write(
		kind: impl Into<SmolStr>,
		name: impl Into<String>,
	) -> Self {
		Self {
			kind: kind.into(),
			name: name.into(),
			permissions: AccessPermissions::ReadWrite,
		}
	}
}

/// What a grant permits, lowered by the compute to its provider's action sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPermissions {
	Read,
	ReadWrite,
}

/// Every grant the blocks of one stack declared, collected once per deploy and
/// handed to the compute blocks that lower them.
///
/// Deduplicated in declaration order at construction, since declaration order is
/// the deploy's order: a policy renders identically across runs and a plan shows
/// no spurious diff.
#[derive(Debug, Default, Clone, Deref)]
pub struct AccessGrants(Vec<AccessGrant>);

impl AccessGrants {
	pub fn new(grants: Vec<AccessGrant>) -> Self {
		let mut out = Vec::<AccessGrant>::new();
		for grant in grants {
			if !out.contains(&grant) {
				out.push(grant);
			}
		}
		Self(out)
	}
}
