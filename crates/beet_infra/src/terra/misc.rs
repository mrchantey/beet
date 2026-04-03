//! Core Terraform/OpenTofu types and traits.
//!
//! These form the typed foundation of beet_infra:
//! - [`Provider`] identifies a provider (AWS, Cloudflare, etc.)
//! - [`ToJson`] converts a value to Terraform-compatible JSON
//! - [`Resource`] marks a struct as a typed Terraform resource

use beet_core::prelude::*;
use serde_json::Value;
use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// A Terraform/OpenTofu provider definition.
///
/// Use the built-in constants ([`Provider::AWS`], [`Provider::CLOUDFLARE`])
/// for well-known providers, or construct a custom one with [`Provider::new`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Provider {
	/// Human-readable display name (e.g. "Amazon Web Services").
	pub name: Cow<'static, str>,
	/// Full registry source path (e.g. "registry.opentofu.org/hashicorp/aws").
	pub source: Cow<'static, str>,
	/// Version constraint (e.g. "~> 6.0").
	pub version: Cow<'static, str>,
}

impl Provider {
	/// Amazon Web Services provider.
	pub const AWS: Self = Self {
		name: Cow::Borrowed("Amazon Web Services"),
		source: Cow::Borrowed("registry.opentofu.org/hashicorp/aws"),
		version: Cow::Borrowed("~> 6.0"),
	};

	/// Cloudflare provider.
	pub const CLOUDFLARE: Self = Self {
		name: Cow::Borrowed("Cloudflare"),
		source: Cow::Borrowed("registry.opentofu.org/cloudflare/cloudflare"),
		version: Cow::Borrowed("~> 5.0"),
	};

	/// Create a custom provider definition.
	pub fn new(
		name: impl Into<String>,
		source: impl Into<String>,
		version: impl Into<String>,
	) -> Self {
		Self {
			name: Cow::Owned(name.into()),
			source: Cow::Owned(source.into()),
			version: Cow::Owned(version.into()),
		}
	}

	/// The local name used in `required_providers` blocks (last segment of source).
	///
	/// ```ignore
	/// terra::Provider::AWS.local_name().xpect_eq("aws");
	/// terra::Provider::CLOUDFLARE.local_name().xpect_eq("cloudflare");
	/// ```
	pub fn local_name(&self) -> &str {
		self.source.split('/').last().unwrap_or(&self.source)
	}

	/// The short source string for `required_providers` (strips the registry prefix).
	///
	/// ```ignore
	/// terra::Provider::AWS.short_source().xpect_eq("hashicorp/aws");
	/// ```
	pub fn short_source(&self) -> &str {
		self.source
			.strip_prefix("registry.opentofu.org/")
			.unwrap_or(&self.source)
	}
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Convert a value into Terraform-compatible JSON.
pub trait ToJson {
	fn to_json(&self) -> Value;
}

/// A typed Terraform resource.
///
/// Every generated resource struct implements this trait, giving the config
/// exporter enough information to automatically:
/// - determine the Terraform resource type (e.g. `"aws_lambda_function"`)
/// - determine which provider is required
/// - serialize the resource body to JSON
/// - validate that required fields are set and computed-only fields are empty
pub trait Resource: ToJson {
	/// The Terraform resource type identifier (e.g. `"aws_lambda_function"`).
	fn resource_type(&self) -> &'static str;

	/// The provider this resource belongs to.
	fn provider(&self) -> &'static Provider;

	/// Validate that all required fields are set and all computed-only fields
	/// are empty. Generated implementations check each field; the default
	/// implementation accepts any state.
	fn validate_definition(&self) -> Result { Ok(()) }
}

/// Applied to resources that have an associated name, like a bucket or iam role.
pub trait PrimaryResource: Resource {
	fn set_primary_identifier(&mut self, name: &str);
}

/// A typed Terraform data source.
///
/// Mirror of [`Resource`] for `data` blocks. Referenced in expressions
/// as `data.<type>.<name>.<attribute>`.
pub trait DataSource: ToJson {
	/// The Terraform data source type (e.g. `"aws_iam_policy_document"`).
	fn data_type(&self) -> &'static str;

	/// The provider this data source belongs to.
	fn provider(&self) -> &'static Provider;
}

/// A typed Terraform backend configuration.
///
/// Implement this trait and pass the backend to
/// [`Config::with_backend`] to configure remote state storage.
///
/// ```ignore
/// let config = terra::Config::new()
///     .with_backend(&S3Backend::default());
/// ```
pub trait Backend {
	/// The backend type identifier, ie `"s3"`, `"local"`, `"gcs"`.
	fn backend_type(&self) -> &'static str;

	/// Serialize the backend configuration body to JSON.
	fn to_backend_json(&self) -> Value;
}

// ---------------------------------------------------------------------------
// ResourceFilter
// ---------------------------------------------------------------------------

/// Selects which resources to include when parsing a provider schema.
///
/// Without a filter the full schema is parsed, which for AWS alone produces
/// an enormous amount of types. Use this to cherry-pick only the resources
/// your project actually needs.
///
/// ```ignore
/// let filter = ResourceFilter::default()
///     .with_resources("registry.opentofu.org/hashicorp/aws", [
///         "aws_lambda_function",
///         "aws_s3_bucket",
///     ])
///     .with_resources("registry.opentofu.org/cloudflare/cloudflare", [
///         "cloudflare_dns_record",
///     ]);
/// ```
#[derive(Clone, Debug)]
pub struct ResourceFilter {
	filters: HashMap<String, HashSet<String>>,
}

impl Default for ResourceFilter {
	fn default() -> Self {
		Self {
			filters: HashMap::new(),
		}
	}
}

impl ResourceFilter {
	/// Add a set of resource type names for a given provider source.
	///
	/// Can be called multiple times — resources are accumulated.
	pub fn with_resources(
		mut self,
		provider: impl Into<String>,
		resources: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		self.filters
			.entry(provider.into())
			.or_default()
			.extend(resources.into_iter().map(Into::into));
		self
	}

	/// Returns `true` when no filters have been configured (allow everything).
	pub fn is_empty(&self) -> bool { self.filters.is_empty() }

	/// Check whether a specific provider is included in the filter at all.
	pub fn has_provider(&self, provider: &str) -> bool {
		if self.filters.is_empty() {
			return true;
		}
		self.filters.contains_key(provider)
	}

	/// Check whether a specific resource should be included.
	///
	/// When the filter is empty every resource is allowed.
	pub fn allows(&self, provider: &str, resource: &str) -> bool {
		if self.filters.is_empty() {
			return true; // no filter => allow all
		}
		self.filters
			.get(provider)
			.map(|rs| rs.contains(resource))
			.unwrap_or(false)
	}

	/// Iterate over all `(provider_source, resource_names)` pairs.
	pub fn iter(&self) -> impl Iterator<Item = (&String, &HashSet<String>)> {
		self.filters.iter()
	}
}

// ---------------------------------------------------------------------------
// ResourceMeta — bookkeeping emitted alongside generated code
// ---------------------------------------------------------------------------

/// Metadata about a generated resource type, used by the code generator to
/// emit trait implementations and by the config generator to wire things up.
#[derive(Clone, Debug)]
pub struct ResourceMeta {
	/// The original Terraform resource type (e.g. `"aws_lambda_function"`).
	pub resource_type: String,
	/// The full provider source (e.g. `"registry.opentofu.org/hashicorp/aws"`).
	pub provider_source: String,
	/// The Rust struct name in the generated output (e.g. `"AwsLambdaFunctionDetails"`).
	pub struct_name: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn provider_local_name() {
		Provider::AWS.local_name().xpect_eq("aws");
		Provider::CLOUDFLARE.local_name().xpect_eq("cloudflare");
	}

	#[test]
	fn provider_short_source() {
		Provider::AWS.short_source().xpect_eq("hashicorp/aws");
		Provider::CLOUDFLARE
			.short_source()
			.xpect_eq("cloudflare/cloudflare");
	}

	#[test]
	fn custom_provider() {
		let provider = Provider::new(
			"My Provider",
			"registry.opentofu.org/acme/thing",
			"~> 1.0",
		);
		provider.local_name().xpect_eq("thing");
		provider.short_source().xpect_eq("acme/thing");
	}

	#[test]
	fn resource_filter_empty_allows_all() {
		let filter = ResourceFilter::default();
		filter.is_empty().xpect_true();
		filter.allows("any_provider", "any_resource").xpect_true();
	}

	#[test]
	fn resource_filter_restricts() {
		let filter = ResourceFilter::default().with_resources(
			"registry.opentofu.org/hashicorp/aws",
			["aws_s3_bucket", "aws_lambda_function"],
		);
		filter.is_empty().xpect_false();
		filter
			.allows("registry.opentofu.org/hashicorp/aws", "aws_s3_bucket")
			.xpect_true();
		filter
			.allows(
				"registry.opentofu.org/hashicorp/aws",
				"aws_lambda_function",
			)
			.xpect_true();
		filter
			.allows("registry.opentofu.org/hashicorp/aws", "aws_ec2_instance")
			.xpect_false();
		filter
			.allows(
				"registry.opentofu.org/cloudflare/cloudflare",
				"cloudflare_dns_record",
			)
			.xpect_false();
	}

	#[test]
	fn resource_filter_accumulates() {
		let filter = ResourceFilter::default()
			.with_resources("p1", ["r1"])
			.with_resources("p1", ["r2"])
			.with_resources("p2", ["r3"]);
		filter.allows("p1", "r1").xpect_true();
		filter.allows("p1", "r2").xpect_true();
		filter.allows("p2", "r3").xpect_true();
		filter.allows("p2", "r1").xpect_false();
	}
}
