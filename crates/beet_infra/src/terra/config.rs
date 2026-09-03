//! Export Terraform/OpenTofu configurations as JSON.
//!
//! The [`Config`] collects provider configurations, resources, data sources,
//! variables, outputs, locals, and backend settings, then serialises them into
//! valid Terraform JSON configuration.
//!
//! # Typed API
//!
//! When using generated provider bindings that implement [`Resource`] or
//! [`DataSource`], the config automatically tracks required providers
//! and serialises bodies with full type safety:
//!
//! ```rust,ignore
//! let bucket = AwsS3BucketDetails { bucket: Some("my-bucket".into()), ..Default::default() };
//! let config = Config::new()
//!     .with_backend(&S3Backend::default())
//!     .with_required_version("~> 1.8")
//!     .with_resource("assets", &bucket)?
//!     .with_output("bucket_name", Output {
//!         value: "${aws_s3_bucket.assets.bucket}".into(),
//!         description: Some("The bucket name".into()),
//!         sensitive: None,
//!     })?;
//! config.export_to_file("main.tf.json").await?;
//! ```
//!
//! # Untyped API
//!
//! The `add_untyped_*` methods accept any `Serialize` type for escape-hatch
//! usage:
//!
//! ```rust,ignore
//! let mut config = Config::new();
//! config.add_required_provider("aws", "hashicorp/aws", "~> 6.0")?;
//! config.add_untyped_provider("aws", &json!({"region": "us-west-2"}))?;
//! config.add_untyped_resource("aws_instance", "web", &my_instance)?;
//! config.export_to_file("main.tf.json").await?;
//! ```

use crate::prelude::*;
use crate::terra::Resource;
use crate::terra::*;
use beet_core::prelude::*;
use serde::Serialize;

/// A Terraform variable definition.
pub struct Variable {
	pub r#type: Option<String>,
	pub default: Option<Value>,
	pub description: Option<String>,
	/// Redacts the value from plan/apply/state-list output. Set for anything
	/// an `EnsureSecret`-style action feeds through, and for the state
	/// encryption passphrase.
	pub sensitive: Option<bool>,
}

/// A Terraform output definition.
pub struct Output {
	pub value: Value,
	pub description: Option<String>,
	pub sensitive: Option<bool>,
}

/// Builds and exports a complete Terraform JSON configuration.
///
/// Each `with_*` method takes ownership and returns `Self` for builder chaining.
/// Each `add_*` / `set_*` method takes `&mut self` for incremental construction.
///
/// # Example
/// ```rust,ignore
/// let config = Config::new()
///     .with_backend(&S3Backend::default())
///     .with_required_version("~> 1.8")
///     .with_resource("assets", &bucket)?
///     .with_output("bucket_name", Output {
///         value: "${aws_s3_bucket.assets.bucket}".into(),
///         description: Some("The bucket name".into()),
///         sensitive: None,
///     })?;
/// config.export_and_validate("infra/main.tf.json").await?;
/// ```
#[derive(Debug, Default, Clone)]
pub struct Config {
	/// Backend for remote state, serialised into `terraform.backend`.
	backend: Option<Value>,
	/// State/plan encryption, serialised into `terraform.encryption`. See
	/// [`StateEncryption`].
	encryption: Option<Value>,
	/// Optional `required_version` constraint in the `terraform` block.
	required_version: Option<String>,
	required_providers: Map,
	/// Provider config blocks. Values are [`Value::Map`] for a single config
	/// or [`Value::List`] of maps when aliases are used.
	providers: Map,
	resources: Map,
	data_sources: Map,
	variables: Map,
	outputs: Map,
	locals: Map,
	/// Resource addresses grouped by deploy layer, collected by
	/// [`add_layer_resource`](Self::add_layer_resource). Config-only, never
	/// serialized.
	layers: HashMap<SmolStr, Vec<String>>,
}

impl Config {
	/// The conventional layer for resources a deploy publishes *into*: a bucket
	/// it syncs, an image registry it pushes to, a table the runtime writes.
	/// Blocks default such resources here so `<TofuApply layer="storage"/>`
	/// converges them before the content upload and the service roll, and expose
	/// the assignment as a field so a route can re-cut its layers.
	pub const STORAGE_LAYER: &str = "storage";

	/// Create a new empty configuration.
	pub fn new() -> Self { Self::default() }

	// =====================================================================
	// Terraform block settings
	// =====================================================================

	/// Set the backend for remote state storage (chaining).
	///
	/// ```ignore
	/// let config = Config::new()
	///     .with_backend(&S3Backend::default());
	/// ```
	pub fn with_backend(mut self, backend: Value) -> Self {
		self.set_backend(backend);
		self
	}

	/// Set the backend for remote state storage.
	pub fn set_backend(&mut self, backend: Value) -> &mut Self {
		self.backend = Some(backend);
		self
	}

	/// Enable state/plan encryption (chaining). A no-op for
	/// [`StateEncryption::None`]; otherwise sets the `terraform.encryption`
	/// block and declares the sensitive `tf_state_passphrase` variable its
	/// key provider reads, with no default so the value must come from a
	/// `-var` at every state-touching invocation (see
	/// [`StateEncryption::vars`]).
	pub fn with_state_encryption(
		mut self,
		encryption: &StateEncryption,
	) -> Self {
		self.set_state_encryption(encryption);
		self
	}

	/// Enable state/plan encryption. See [`Self::with_state_encryption`].
	pub fn set_state_encryption(
		&mut self,
		encryption: &StateEncryption,
	) -> &mut Self {
		if let Some(json) = encryption.to_json() {
			self.encryption = Some(json);
			self.ensure_variable(STATE_ENCRYPTION_VAR, Variable {
				r#type: Some("string".into()),
				default: None,
				description: Some(
					"OpenTofu state/plan encryption passphrase, supplied via -var at apply time, never defaulted."
						.into(),
				),
				sensitive: Some(true),
			});
		}
		self
	}

	/// Set the required OpenTofu/Terraform version constraint (chaining).
	///
	/// ```ignore
	/// let config = Config::new().with_required_version("~> 1.8");
	/// ```
	pub fn with_required_version(
		mut self,
		constraint: impl Into<String>,
	) -> Self {
		self.set_required_version(constraint);
		self
	}

	/// Set the required OpenTofu/Terraform version constraint.
	pub fn set_required_version(
		&mut self,
		constraint: impl Into<String>,
	) -> &mut Self {
		self.required_version = Some(constraint.into());
		self
	}

	// =====================================================================
	// Typed resource / data-source API
	// =====================================================================

	pub fn with_resource<T: Resource>(
		mut self,
		resource: &terra::ResourceDef<T>,
	) -> Result<Self> {
		self.add_resource(resource)?;
		Ok(self)
	}

	/// a slug is able generate both the label and resource name,
	/// creating a a shorthand for resources that are [`SetSlug`]
	pub fn add_resource<T: Resource>(
		&mut self,
		resource: &terra::ResourceDef<T>,
	) -> Result<&mut Self> {
		self.add_labeled_resource(resource.ident().label(), resource.resource())
	}

	/// Add a typed resource assigned to a named deploy layer, a set of `-target`
	/// addresses a route converges ahead of the full apply via
	/// `<TofuApply layer="..."/>`.
	///
	/// Layers are milestones through the one stack graph rather than partitions
	/// of it: a targeted apply pulls in each target's dependencies, so a layer
	/// need not be dependency-closed. Blocks default their publish-into resources
	/// to [`STORAGE_LAYER`](Self::STORAGE_LAYER).
	pub fn add_layer_resource<T: Resource>(
		&mut self,
		layer: impl Into<SmolStr>,
		resource: &terra::ResourceDef<T>,
	) -> Result<&mut Self> {
		self.add_resource(resource)?;
		self.layers
			.entry(layer.into())
			.or_default()
			.push(resource.address());
		Ok(self)
	}

	/// The addresses declared under `layer`, as `tofu apply -target` takes them.
	/// Non-empty by construction, since only
	/// [`add_layer_resource`](Self::add_layer_resource) creates an entry.
	///
	/// An unknown layer is a loud error naming the declared layers: a typo that
	/// silently converged nothing would race exactly as an unordered deploy does.
	pub fn layer_targets(&self, layer: &str) -> Result<&[String]> {
		self.layers
			.get(layer)
			.map(|targets| targets.as_slice())
			.ok_or_else(|| {
				let mut declared =
					self.layers.keys().cloned().collect::<Vec<_>>();
				declared.sort();
				bevyhow!(
					"no resources declare layer '{layer}', declared layers: {declared:?}"
				)
			})
	}

	/// Add a typed resource (chaining). The required provider is registered
	/// automatically from the resource's [`Resource`] implementation.
	pub fn with_labeled_resource(
		mut self,
		name: impl Into<String>,
		resource: &dyn Resource,
	) -> Result<Self> {
		self.add_labeled_resource(name, resource)?;
		Ok(self)
	}

	/// Add a typed resource. The required provider is registered automatically
	/// from the resource's [`Resource`] implementation.
	/// ## Errors
	///
	/// - If the resource is invalid, ie [`Resource::validate_definition`]
	/// - If an existing resource with the provided label
	pub fn add_labeled_resource(
		&mut self,
		label: impl Into<String>,
		resource: &dyn Resource,
	) -> Result<&mut Self> {
		let label = label.into();
		resource.validate_definition()?;
		self.ensure_provider(resource.provider());
		let map = self
			.resources
			.entry(resource.resource_type().into())
			.or_insert_with(Value::map)
			.as_map_mut()?;
		if map.insert(label.clone(), resource.to_json()).is_some() {
			bevybail!(
				"duplicate resource: type `{}` label `{}` already exists",
				resource.resource_type(),
				label
			);
		}
		Ok(self)
	}

	/// Add a typed data source (chaining). The required provider is registered
	/// automatically from the data source's [`DataSource`] implementation.
	pub fn with_data_source(
		mut self,
		name: impl Into<String>,
		source: &dyn DataSource,
	) -> Result<Self> {
		self.add_data_source_typed(name, source)?;
		self.xok()
	}

	/// Add a typed data source. The required provider is registered automatically
	/// from the data source's [`DataSource`] implementation.
	pub fn add_data_source_typed(
		&mut self,
		label: impl Into<String>,
		source: &dyn DataSource,
	) -> Result<&mut Self> {
		let label = label.into();
		self.ensure_provider(source.provider());
		let map = self
			.data_sources
			.entry(source.data_type().into())
			.or_insert_with(Value::map)
			.as_map_mut()?;
		if map.insert(label.clone(), source.to_json()).is_some() {
			bevybail!(
				"duplicate data source: type `{}` label `{}` already exists",
				source.data_type(),
				label
			);
		}
		self.xok()
	}

	// =====================================================================
	// Typed provider API
	// =====================================================================

	/// Add a provider configuration block (chaining).
	///
	/// The provider is auto-registered in `required_providers` if not already
	/// present. For multiple configs (aliases) use [`with_provider_alias`].
	pub fn with_provider_config(
		mut self,
		provider: &Provider,
		config: &impl Serialize,
	) -> Result<Self> {
		self.add_provider_config(provider, config)?;
		Ok(self)
	}

	/// Add a provider configuration block.
	///
	/// The provider is auto-registered in `required_providers` if not already present.
	pub fn add_provider_config(
		&mut self,
		provider: &Provider,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		self.ensure_provider(provider);
		let value = Value::from_serde(config)?;
		self.insert_provider_entry(provider.local_name(), value)?;
		Ok(self)
	}

	/// Add a provider configuration block only if one is not already present.
	/// Unlike [`Config::add_provider_config`], silently succeeds when the
	/// provider already has a config, so several blocks can each ensure a
	/// shared provider (eg Cloudflare for DNS and ACM validation records)
	/// without producing a spurious aliased array.
	pub fn ensure_provider_config(
		&mut self,
		provider: &Provider,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		if self.providers.contains(provider.local_name()) {
			return Ok(self);
		}
		self.add_provider_config(provider, config)
	}

	/// Add an aliased provider configuration block (chaining).
	///
	/// Use this when you need multiple configurations for the same provider,
	/// eg two AWS regions. The `alias` field is injected automatically.
	/// Calling this a second time for the same provider upgrades the block to
	/// an array, which is the correct Terraform JSON format for aliases.
	///
	/// ```ignore
	/// let config = Config::new()
	///     .with_provider_config(&Provider::AWS, &json!({"region": "us-east-1"}))?
	///     .with_provider_alias(&Provider::AWS, "eu_west_1", &json!({"region": "eu-west-1"}))?;
	/// ```
	pub fn with_provider_alias(
		mut self,
		provider: &Provider,
		alias: impl Into<String>,
		config: &impl Serialize,
	) -> Result<Self> {
		self.add_provider_alias(provider, alias, config)?;
		Ok(self)
	}

	/// Add an aliased provider configuration block.
	///
	/// See [`with_provider_alias`] for details.
	pub fn add_provider_alias(
		&mut self,
		provider: &Provider,
		alias: impl Into<String>,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		self.ensure_provider(provider);
		let mut value = Value::from_serde(config)?;
		let alias: String = alias.into();
		if let Value::Map(map) = &mut value {
			map.insert("alias", alias);
		}
		self.insert_provider_entry(provider.local_name(), value)?;
		Ok(self)
	}

	// =====================================================================
	// Variables / outputs / locals
	// =====================================================================

	/// Add a variable definition (chaining).
	/// ## Errors
	/// - If a variable with the same name already exists
	pub fn with_variable(
		mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> Result<Self> {
		self.insert_variable(name, variable)?;
		Ok(self)
	}

	/// Add a variable definition.
	/// ## Errors
	/// - If a variable with the same name already exists
	pub fn add_variable(
		&mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> Result<&mut Self> {
		self.insert_variable(name, variable)?;
		Ok(self)
	}

	/// Add a variable declaration if one with this name doesn't already exist.
	/// Unlike [`Config::add_variable`], silently succeeds on duplicates.
	pub fn ensure_variable(
		&mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> &mut Self {
		let name = name.into();
		if !self.variables.contains(&name) {
			self.insert_variable(name, variable).ok();
		}
		self
	}

	/// Add an output definition (chaining).
	/// ## Errors
	/// - If an output with the same name already exists
	pub fn with_output(
		mut self,
		name: impl Into<String>,
		output: Output,
	) -> Result<Self> {
		self.insert_output(name, output)?;
		Ok(self)
	}

	/// Add an output definition.
	/// ## Errors
	/// - If an output with the same name already exists
	pub fn add_output(
		&mut self,
		name: impl Into<String>,
		output: Output,
	) -> Result<&mut Self> {
		self.insert_output(name, output)?;
		Ok(self)
	}

	/// Add a local value (chaining).
	pub fn with_local(
		mut self,
		name: impl Into<String>,
		value: impl Serialize,
	) -> Result<Self> {
		self.add_local(name, value)?;
		Ok(self)
	}

	/// Add a local value.
	pub fn add_local(
		&mut self,
		name: impl Into<String>,
		value: impl Serialize,
	) -> Result<&mut Self> {
		let name: String = name.into();
		self.locals.insert(name, Value::from_serde(value)?);
		Ok(self)
	}

	// =====================================================================
	// Untyped / escape-hatch API
	// =====================================================================

	/// Add a required provider declaration without a typed [`Provider`].
	/// ## Errors
	/// - If a provider with the same name already exists
	pub fn add_required_provider(
		&mut self,
		name: &str,
		source: &str,
		version: &str,
	) -> Result<&mut Self> {
		if self.required_providers.contains(name) {
			bevybail!("duplicate required provider: `{}` already exists", name);
		}
		self.required_providers
			.insert(name, value!({ "source": source, "version": version }));
		Ok(self)
	}

	/// Add a raw provider configuration block by name.
	pub fn add_untyped_provider(
		&mut self,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		let value = Value::from_serde(config)?;
		self.insert_provider_entry(name, value)?;
		Ok(self)
	}

	/// Add a raw resource block.
	pub fn add_untyped_resource(
		&mut self,
		resource_type: &str,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		let value = Value::from_serde(config)?;
		self.resources
			.entry(resource_type.into())
			.or_insert_with(Value::map)
			.as_map_mut()?
			.insert(name, value);
		Ok(self)
	}

	/// Add a raw data source block.
	pub fn add_untyped_data_source(
		&mut self,
		data_type: &str,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		let value = Value::from_serde(config)?;
		self.data_sources
			.entry(data_type.into())
			.or_insert_with(Value::map)
			.as_map_mut()?
			.insert(name, value);
		Ok(self)
	}

	/// Inject a `lifecycle` block into an already-added resource.
	/// The resource must have been added before calling this method.
	///
	/// `resource_type` and `label` identify the target resource,
	/// and `lifecycle` is the JSON value for the lifecycle block, ie:
	/// ```ignore
	/// json!({ "replace_triggered_by": ["aws_lightsail_instance.xxx"] })
	/// ```
	pub fn set_lifecycle(
		&mut self,
		resource_type: &str,
		label: &str,
		lifecycle: impl Serialize,
	) -> Result<&mut Self> {
		let map = self
			.resources
			.get_mut(resource_type)
			.and_then(|v| v.as_map_mut().ok())
			.ok_or_else(|| {
				bevyhow!("resource type `{resource_type}` not found")
			})?;
		let resource = map
			.get_mut(label)
			.and_then(|v| v.as_map_mut().ok())
			.ok_or_else(|| {
				bevyhow!("resource `{resource_type}.{label}` not found")
			})?;
		resource.insert("lifecycle", Value::from_serde(lifecycle)?);
		Ok(self)
	}

	// =====================================================================
	// Serialization
	// =====================================================================

	/// Build the complete Terraform JSON configuration as a [`Value`].
	pub fn to_json(&self) -> Value {
		let mut root = Map::default();

		// terraform block: optional required_version, backend, required_providers
		let mut tf_block = Map::default();
		if let Some(version) = &self.required_version {
			tf_block.insert("required_version", version.clone());
		}
		if let Some(backend) = &self.backend {
			tf_block.insert("backend", backend.clone());
		}
		if let Some(encryption) = &self.encryption {
			tf_block.insert("encryption", encryption.clone());
		}
		if !self.required_providers.is_empty() {
			tf_block
				.insert("required_providers", self.required_providers.clone());
		}
		if !tf_block.is_empty() {
			root.insert("terraform", tf_block);
		}

		if !self.providers.is_empty() {
			root.insert("provider", self.providers.clone());
		}
		if !self.variables.is_empty() {
			root.insert("variable", self.variables.clone());
		}
		if !self.locals.is_empty() {
			root.insert("locals", self.locals.clone());
		}
		if !self.resources.is_empty() {
			root.insert("resource", self.resources.clone());
		}
		if !self.data_sources.is_empty() {
			root.insert("data", self.data_sources.clone());
		}
		if !self.outputs.is_empty() {
			root.insert("output", self.outputs.clone());
		}

		Value::Map(root)
	}

	/// Serialize [`Self::to_json`] to a compact JSON string, entries sorted
	/// by key.
	pub fn to_json_string(&self) -> Result<String> {
		serde_json::to_string(&self.to_json())?.xok()
	}

	// =====================================================================
	// Internal helpers
	// =====================================================================

	/// Register a provider in `required_providers` if not already present.
	fn ensure_provider(&mut self, provider: &Provider) {
		let local = provider.local_name();
		if self.required_providers.contains(local) {
			return;
		}
		let source = provider.short_source();
		let version = provider.version.as_ref();
		self.required_providers.insert(
			SmolStr::from(local),
			value!({ "source": source, "version": version }),
		);
	}

	/// Insert a provider config, upgrading to an array on the second call
	/// for the same provider (required by Terraform for aliased providers).
	fn insert_provider_entry(
		&mut self,
		local_name: &str,
		config: Value,
	) -> Result<()> {
		match self.providers.get_mut(local_name) {
			None => {
				self.providers.insert(local_name, config);
			}
			Some(existing @ Value::Map(_)) => {
				let first = existing.clone();
				*existing = Value::List(vec![first, config]);
			}
			Some(Value::List(list)) => {
				list.push(config);
			}
			Some(other) => {
				other.as_map_mut()?;
			}
		}
		Ok(())
	}

	fn insert_variable(
		&mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> Result<()> {
		let name: String = name.into();
		if self.variables.contains(&name) {
			bevybail!("duplicate variable: `{}` already exists", name);
		}
		let mut obj = Map::default();
		if let Some(var_type) = variable.r#type {
			obj.insert("type", var_type);
		}
		if let Some(default) = variable.default {
			obj.insert("default", default);
		}
		if let Some(desc) = variable.description {
			obj.insert("description", desc);
		}
		if let Some(sensitive) = variable.sensitive {
			obj.insert("sensitive", sensitive);
		}
		self.variables.insert(name, obj);
		Ok(())
	}

	fn insert_output(
		&mut self,
		name: impl Into<String>,
		output: Output,
	) -> Result<()> {
		let name: String = name.into();
		if self.outputs.contains(&name) {
			bevybail!("duplicate output: `{}` already exists", name);
		}
		let mut obj = Map::default();
		obj.insert("value", output.value);
		if let Some(desc) = output.description {
			obj.insert("description", desc);
		}
		if let Some(sensitive) = output.sensitive {
			obj.insert("sensitive", sensitive);
		}
		self.outputs.insert(name, obj);
		Ok(())
	}
}
