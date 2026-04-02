//! Export Terraform/OpenTofu configurations as JSON.
//!
//! The [`ConfigExporter`] collects provider configurations, resources, data sources,
//! variables, outputs, locals, and backend settings, then serialises them into
//! valid Terraform JSON configuration.
//!
//! # Typed API
//!
//! When using generated provider bindings that implement [`TerraResource`] or
//! [`TerraDataSource`], the exporter automatically tracks required providers
//! and serialises bodies with full type safety:
//!
//! ```rust,ignore
//! let bucket = AwsS3BucketDetails { bucket: Some("my-bucket".into()), ..Default::default() };
//! let exporter = ConfigExporter::new()
//!     .with_backend(&S3Backend::default())
//!     .with_required_version("~> 1.8")
//!     .with_resource("assets", &bucket)
//!     .with_output("bucket_name", Output {
//!         value: json!("${aws_s3_bucket.assets.bucket}"),
//!         description: Some("The bucket name".into()),
//!         sensitive: None,
//!     });
//! exporter.export_to_file("main.tf.json").await?;
//! ```
//!
//! # Untyped API
//!
//! The `add_untyped_*` methods accept raw `serde_json::Value` or any
//! `Serialize` type for escape-hatch usage:
//!
//! ```rust,ignore
//! let mut exporter = ConfigExporter::new();
//! exporter.add_required_provider("aws", "hashicorp/aws", "~> 6.0");
//! exporter.add_untyped_provider("aws", &json!({"region": "us-west-2"}))?;
//! exporter.add_untyped_resource("aws_instance", "web", &my_instance)?;
//! exporter.export_to_file("main.tf.json").await?;
//! ```

use super::misc::TerraBackend;
use super::misc::TerraDataSource;
use super::misc::TerraProvider;
use super::misc::TerraResource;
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
/// let exporter = ConfigExporter::new()
///     .with_backend(&S3Backend::default())
///     .with_required_version("~> 1.8")
///     .with_resource("assets", &bucket)
///     .with_output("bucket_name", Output {
///         value: json!("${aws_s3_bucket.assets.bucket}"),
///         description: Some("The bucket name".into()),
///         sensitive: None,
///     });
/// exporter.export_and_validate("infra/main.tf.json").await?;
/// ```
pub struct ConfigExporter {
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

impl ConfigExporter {
	/// Create a new empty configuration exporter.
	pub fn new() -> Self {
		Self {
			backend: None,
			required_version: None,
			required_providers: Map::new(),
			providers: Map::new(),
			resources: Map::new(),
			data_sources: Map::new(),
			variables: Map::new(),
			outputs: Map::new(),
			locals: Map::new(),
		}
	}

	// =====================================================================
	// Terraform block settings
	// =====================================================================

	/// Set the backend for remote state storage (chaining).
	///
	/// ```ignore
	/// let exporter = ConfigExporter::new()
	///     .with_backend(&S3Backend::default());
	/// ```
	pub fn with_backend(mut self, backend: &dyn TerraBackend) -> Self {
		self.set_backend(backend);
		self
	}

	/// Set the backend for remote state storage.
	pub fn set_backend(&mut self, backend: &dyn TerraBackend) -> &mut Self {
		self.backend = Some((
			backend.backend_type().to_string(),
			backend.to_backend_json(),
		));
		self
	}

	/// Set the required OpenTofu/Terraform version constraint (chaining).
	///
	/// ```ignore
	/// let exporter = ConfigExporter::new().with_required_version("~> 1.8");
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

	/// Add a typed resource (chaining). The required provider is registered
	/// automatically from the resource's [`TerraResource`] implementation.
	pub fn with_resource(
		mut self,
		name: impl Into<String>,
		resource: &dyn TerraResource,
	) -> Self {
		self.add_resource(name, resource);
		self
	}

	/// Add a typed resource. The required provider is registered automatically
	/// from the resource's [`TerraResource`] implementation.
	pub fn add_resource(
		&mut self,
		name: impl Into<String>,
		resource: &dyn TerraResource,
	) -> &mut Self {
		self.ensure_provider(resource.provider());
		let type_map = self
			.resources
			.entry(resource.resource_type().to_string())
			.or_insert_with(|| Value::Object(Map::new()));
		if let Value::Object(map) = type_map {
			map.insert(name.into(), resource.to_json());
		}
		self
	}

	/// Add a typed data source (chaining). The required provider is registered
	/// automatically from the data source's [`TerraDataSource`] implementation.
	pub fn with_data_source(
		mut self,
		name: impl Into<String>,
		source: &dyn TerraDataSource,
	) -> Self {
		self.add_data_source_typed(name, source);
		self
	}

	/// Add a typed data source. The required provider is registered automatically
	/// from the data source's [`TerraDataSource`] implementation.
	pub fn add_data_source_typed(
		&mut self,
		name: impl Into<String>,
		source: &dyn TerraDataSource,
	) -> &mut Self {
		self.ensure_provider(source.provider());
		let type_map = self
			.data_sources
			.entry(source.data_type().to_string())
			.or_insert_with(|| Value::Object(Map::new()));
		if let Value::Object(map) = type_map {
			map.insert(name.into(), source.to_json());
		}
		self
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
		provider: &TerraProvider,
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
		provider: &TerraProvider,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		self.ensure_provider(provider);
		let value = serde_json::to_value(config)?;
		self.insert_provider_entry(provider.local_name(), value);
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
	/// let exporter = ConfigExporter::new()
	///     .with_provider_config(&TerraProvider::AWS, &json!({"region": "us-east-1"}))?
	///     .with_provider_alias(&TerraProvider::AWS, "eu_west_1", &json!({"region": "eu-west-1"}))?;
	/// ```
	pub fn with_provider_alias(
		mut self,
		provider: &TerraProvider,
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
		provider: &TerraProvider,
		alias: impl Into<String>,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		self.ensure_provider(provider);
		let mut value = serde_json::to_value(config)?;
		if let Value::Object(ref mut map) = value {
			map.insert("alias".to_string(), Value::String(alias.into()));
		}
		self.insert_provider_entry(provider.local_name(), value);
		Ok(self)
	}

	// =====================================================================
	// Variables / outputs / locals
	// =====================================================================

	/// Add a variable definition (chaining).
	pub fn with_variable(
		mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> Self {
		self.insert_variable(name, variable);
		self
	}

	/// Add a variable definition.
	pub fn add_variable(
		&mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> &mut Self {
		self.insert_variable(name, variable);
		self
	}

	/// Add an output definition (chaining).
	pub fn with_output(
		mut self,
		name: impl Into<String>,
		output: Output,
	) -> Self {
		self.insert_output(name, output);
		self
	}

	/// Add an output definition.
	pub fn add_output(
		&mut self,
		name: impl Into<String>,
		output: Output,
	) -> &mut Self {
		self.insert_output(name, output);
		self
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

	/// Add a required provider declaration without a typed [`TerraProvider`].
	pub fn add_required_provider(
		&mut self,
		name: &str,
		source: &str,
		version: &str,
	) -> &mut Self {
		self.required_providers.insert(
			name.to_string(),
			json!({ "source": source, "version": version }),
		);
		self
	}

	/// Add a raw provider configuration block by name.
	pub fn add_untyped_provider(
		&mut self,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self> {
		let value = serde_json::to_value(config)?;
		self.insert_provider_entry(name, value);
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
		let type_map = self
			.resources
			.entry(resource_type.to_string())
			.or_insert_with(|| Value::Object(Map::new()));
		if let Value::Object(map) = type_map {
			map.insert(name.to_string(), value);
		}
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
		let type_map = self
			.data_sources
			.entry(data_type.to_string())
			.or_insert_with(|| Value::Object(Map::new()));
		if let Value::Object(map) = type_map {
			map.insert(name.to_string(), value);
		}
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
	pub async fn validate(&self) -> Result<String> {
		let dir = TempDir::new()?;
		self.export_to_file(dir.as_ref().join("main.tf.json"))
			.await?;

		let init = async_process::Command::new("tofu")
			.current_dir(dir.as_ref())
			.args(["init"])
			.output()
			.await?;
		if !init.status.success() {
			bevybail!(
				"tofu init failed:\n{}",
				String::from_utf8_lossy(&init.stderr)
			);
		}

		let validate = async_process::Command::new("tofu")
			.current_dir(dir.as_ref())
			.args(["validate", "-json"])
			.output()
			.await?;

		let stdout = String::from_utf8_lossy(&validate.stdout).to_string();
		if !validate.status.success() {
			bevybail!(
				"tofu validate failed:\nstdout: {}\nstderr: {}",
				stdout,
				String::from_utf8_lossy(&validate.stderr),
			);
		}
		Ok(stdout)
	}

	/// Export to `path`, then run `tofu init` + `tofu validate` in-place.
	///
	/// Convenience wrapper combining [`export_to_file`] and [`validate`] that
	/// operates on the given path rather than a temp directory.
	pub async fn export_and_validate(
		&self,
		path: impl AsRef<Path>,
	) -> Result<String> {
		let path = path.as_ref();
		let dir = path.parent().unwrap_or(Path::new("."));
		fs_ext::create_dir_all(dir)?;
		self.export_to_file(path).await?;
		cross_log!("Generated: {}", path.display());

		cross_log!("Running tofu init …");
		let init = async_process::Command::new("tofu")
			.current_dir(dir)
			.args(["init"])
			.output()
			.await?;
		if !init.status.success() {
			bevybail!(
				"tofu init failed:\n{}",
				String::from_utf8_lossy(&init.stderr)
			);
		}
		cross_log!("tofu init: OK");

		cross_log!("Running tofu validate …");
		let validate = async_process::Command::new("tofu")
			.current_dir(dir)
			.args(["validate", "-json"])
			.output()
			.await?;

		let stdout = String::from_utf8_lossy(&validate.stdout).to_string();
		if !validate.status.success() {
			bevybail!(
				"tofu validate failed:\nstdout: {}\nstderr: {}",
				stdout,
				String::from_utf8_lossy(&validate.stderr),
			);
		}
		cross_log!("tofu validate: PASSED");
		Ok(stdout)
	}

	// =====================================================================
	// Internal helpers
	// =====================================================================

	/// Register a provider in `required_providers` if not already present.
	fn ensure_provider(&mut self, provider: &TerraProvider) {
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
	fn insert_provider_entry(&mut self, local_name: &str, config: Value) {
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
			_ => {}
		}
	}

	fn insert_variable(&mut self, name: impl Into<String>, variable: Variable) {
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
		self.variables.insert(name.into(), Value::Object(obj));
	}

	fn insert_output(&mut self, name: impl Into<String>, output: Output) {
		let mut obj = Map::new();
		obj.insert("value".to_string(), output.value);
		if let Some(desc) = output.description {
			obj.insert("description".to_string(), Value::String(desc));
		}
		if let Some(sensitive) = output.sensitive {
			obj.insert("sensitive".to_string(), Value::Bool(sensitive));
		}
		self.outputs.insert(name.into(), Value::Object(obj));
	}
}

impl Default for ConfigExporter {
	fn default() -> Self { Self::new() }
}
