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
//!         value: json!("${aws_s3_bucket.assets.bucket}"),
//!         description: Some("The bucket name".into()),
//!         sensitive: None,
//!     })?;
//! config.export_to_file("main.tf.json").await?;
//! ```
//!
//! # Untyped API
//!
//! The `add_untyped_*` methods accept raw `serde_json::Value` or any
//! `Serialize` type for escape-hatch usage:
//!
//! ```rust,ignore
//! let mut config = Config::new();
//! config.add_required_provider("aws", "hashicorp/aws", "~> 6.0")?;
//! config.add_untyped_provider("aws", &json!({"region": "us-west-2"}))?;
//! config.add_untyped_resource("aws_instance", "web", &my_instance)?;
//! config.export_to_file("main.tf.json").await?;
//! ```

use super::misc::Backend;
use super::misc::DataSource;
use super::misc::Provider;
use super::misc::Resource;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use std::io::Write;
use std::path::Path;

/// A Terraform variable definition.
pub struct Variable {
	pub r#type: Option<String>,
	pub default: Option<Value>,
	pub description: Option<String>,
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
///         value: json!("${aws_s3_bucket.assets.bucket}"),
///         description: Some("The bucket name".into()),
///         sensitive: None,
///     })?;
/// config.export_and_validate("infra/main.tf.json").await?;
/// ```
#[derive(Debug, Default, Clone)]
pub struct Config {
	/// Backend for remote state, serialised into `terraform.backend`.
	backend: Option<(String, Value)>,
	/// Optional `required_version` constraint in the `terraform` block.
	required_version: Option<String>,
	required_providers: Map<String, Value>,
	/// Provider config blocks. Values are `Object` for a single config or
	/// `Array` of objects when aliases are used.
	providers: Map<String, Value>,
	resources: Map<String, Value>,
	data_sources: Map<String, Value>,
	variables: Map<String, Value>,
	outputs: Map<String, Value>,
	locals: Map<String, Value>,
}

impl Config {
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
	pub fn with_backend(mut self, backend: &dyn Backend) -> Self {
		self.set_backend(backend);
		self
	}

	/// Set the backend for remote state storage.
	pub fn set_backend(&mut self, backend: &dyn Backend) -> &mut Self {
		self.backend = Some((
			backend.backend_type().to_string(),
			backend.to_backend_json(),
		));
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
			.entry(resource.resource_type().to_string())
			.or_insert_with(|| Value::Object(Map::new()))
			.to_object_mut()?;
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
			.entry(source.data_type().to_string())
			.or_insert_with(|| Value::Object(Map::new()))
			.to_object_mut()?;
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
		let value = serde_json::to_value(config)?;
		self.insert_provider_entry(provider.local_name(), value)?;
		Ok(self)
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
		let mut value = serde_json::to_value(config)?;
		if let Value::Object(ref mut map) = value {
			map.insert("alias".to_string(), Value::String(alias.into()));
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
		self.locals
			.insert(name.into(), serde_json::to_value(value)?);
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
		if self.required_providers.contains_key(name) {
			bevybail!("duplicate required provider: `{}` already exists", name);
		}
		self.required_providers.insert(
			name.to_string(),
			json!({ "source": source, "version": version }),
		);
		Ok(self)
	}

	/// Add a raw provider configuration block by name.
	pub fn add_untyped_provider(
		&mut self,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		let value = serde_json::to_value(config)?;
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
		let value = serde_json::to_value(config)?;
		self.resources
			.entry(resource_type.to_string())
			.or_insert_with(|| Value::Object(Map::new()))
			.to_object_mut()?
			.insert(name.to_string(), value);
		Ok(self)
	}

	/// Add a raw data source block.
	pub fn add_untyped_data_source(
		&mut self,
		data_type: &str,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		let value = serde_json::to_value(config)?;
		self.data_sources
			.entry(data_type.to_string())
			.or_insert_with(|| Value::Object(Map::new()))
			.to_object_mut()?
			.insert(name.to_string(), value);
		Ok(self)
	}

	// =====================================================================
	// Serialization
	// =====================================================================

	/// Build the complete Terraform JSON configuration as a [`Value`].
	pub fn to_value(&self) -> Value {
		let mut root = Map::new();

		// terraform block: optional required_version, backend, required_providers
		let mut tf_block = Map::new();
		if let Some(ref v) = self.required_version {
			tf_block.insert(
				"required_version".to_string(),
				Value::String(v.clone()),
			);
		}
		if let Some((ref backend_type, ref backend_cfg)) = self.backend {
			tf_block.insert(
				"backend".to_string(),
				json!({ backend_type: backend_cfg }),
			);
		}
		if !self.required_providers.is_empty() {
			tf_block.insert(
				"required_providers".to_string(),
				Value::Object(self.required_providers.clone()),
			);
		}
		if !tf_block.is_empty() {
			root.insert("terraform".to_string(), Value::Object(tf_block));
		}

		if !self.providers.is_empty() {
			root.insert(
				"provider".to_string(),
				Value::Object(self.providers.clone()),
			);
		}
		if !self.variables.is_empty() {
			root.insert(
				"variable".to_string(),
				Value::Object(self.variables.clone()),
			);
		}
		if !self.locals.is_empty() {
			root.insert(
				"locals".to_string(),
				Value::Object(self.locals.clone()),
			);
		}
		if !self.resources.is_empty() {
			root.insert(
				"resource".to_string(),
				Value::Object(self.resources.clone()),
			);
		}
		if !self.data_sources.is_empty() {
			root.insert(
				"data".to_string(),
				Value::Object(self.data_sources.clone()),
			);
		}
		if !self.outputs.is_empty() {
			root.insert(
				"output".to_string(),
				Value::Object(self.outputs.clone()),
			);
		}

		Value::Object(root)
	}

	/// Serialize to a pretty-printed JSON string.
	pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string_pretty(&self.to_value())
	}

	/// Write the configuration to a writer.
	pub fn export_to_writer(&self, writer: &mut dyn Write) -> Result {
		let json = self.to_json_pretty()?;
		writer.write_all(json.as_bytes())?;
		writer.write_all(b"\n")?;
		Ok(())
	}

	/// Write the configuration to a file.
	pub async fn export_to_file(&self, path: impl AsRef<Path>) -> Result {
		let json = self.to_json_pretty()?;
		let mut content = json.into_bytes();
		content.push(b'\n');
		fs_ext::write_async(path, &content).await?;
		Ok(())
	}

	// =====================================================================
	// Validation
	// =====================================================================

	/// Write the config to a temporary directory and run `tofu validate`.
	///
	/// Returns the JSON output of `tofu validate -json` on success.
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn validate(&self) -> Result<String> {
		let dir = TempDir::new()?;
		self.export_to_file(dir.as_ref().join("main.tf.json"))
			.await?;

		tofu::init(dir.path()).await?;
		tofu::validate(dir.path()).await
	}

	/// Export to `path`, then run `tofu init` + `tofu validate` in-place.
	///
	/// Convenience wrapper combining [`export_to_file`] and [`validate`] that
	/// operates on the given path rather than a temp directory.
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn export_and_validate(
		&self,
		path: &AbsPathBuf,
	) -> Result<String> {
		let dir = path.parent().unwrap_or_default();
		fs_ext::create_dir_all(&dir)?;
		self.export_to_file(path).await?;
		cross_log!("Generated: {}", path.display());
		cross_log!("Running tofu init …");
		tofu::init(&dir).await?;
		cross_log!("tofu init: OK");
		cross_log!("Running tofu validate …");
		let stdout = tofu::validate(&dir).await?;

		cross_log!("tofu validate: PASSED");
		Ok(stdout)
	}

	// =====================================================================
	// Internal helpers
	// =====================================================================

	/// Register a provider in `required_providers` if not already present.
	fn ensure_provider(&mut self, provider: &Provider) {
		let local = provider.local_name().to_string();
		if self.required_providers.contains_key(&local) {
			return;
		}
		self.required_providers.insert(
			local,
			json!({
				"source":  provider.short_source(),
				"version": provider.version.as_ref(),
			}),
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
				self.providers.insert(local_name.to_string(), config);
			}
			Some(existing @ Value::Object(_)) => {
				let first = existing.clone();
				*existing = Value::Array(vec![first, config]);
			}
			Some(Value::Array(arr)) => {
				arr.push(config);
			}
			Some(other) => {
				other.to_object_mut()?;
			}
		}
		Ok(())
	}

	fn insert_variable(
		&mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> Result<()> {
		let name = name.into();
		if self.variables.contains_key(&name) {
			bevybail!("duplicate variable: `{}` already exists", name);
		}
		let mut obj = Map::new();
		if let Some(var_type) = variable.r#type {
			obj.insert("type".to_string(), Value::String(var_type));
		}
		if let Some(default) = variable.default {
			obj.insert("default".to_string(), default);
		}
		if let Some(desc) = variable.description {
			obj.insert("description".to_string(), Value::String(desc));
		}
		self.variables.insert(name, Value::Object(obj));
		Ok(())
	}

	fn insert_output(
		&mut self,
		name: impl Into<String>,
		output: Output,
	) -> Result<()> {
		let name = name.into();
		if self.outputs.contains_key(&name) {
			bevybail!("duplicate output: `{}` already exists", name);
		}
		let mut obj = Map::new();
		obj.insert("value".to_string(), output.value);
		if let Some(desc) = output.description {
			obj.insert("description".to_string(), Value::String(desc));
		}
		if let Some(sensitive) = output.sensitive {
			obj.insert("sensitive".to_string(), Value::Bool(sensitive));
		}
		self.outputs.insert(name, Value::Object(obj));
		Ok(())
	}
}
