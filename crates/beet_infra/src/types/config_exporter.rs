//! Export Terraform/OpenTofu configurations as JSON.
//!
//! The [`ConfigExporter`] collects provider configurations, resources, data sources,
//! and variable/output/local definitions, then serializes them into valid
//! Terraform JSON configuration.
//!
//! # Typed API
//!
//! When using generated provider bindings that implement [`TerraResource`], the
//! exporter automatically tracks required providers and serialises resource
//! bodies with full type safety:
//!
//! ```rust,ignore
//! let bucket = AwsS3BucketDetails { bucket: Some("my-bucket".into()), ..Default::default() };
//! let exporter = ConfigExporter::new()
//!     .with_resource("assets", &bucket);
//! ```
//!
//! # Untyped API
//!
//! The `add_untyped_resource` / `add_untyped_provider` methods that accept raw
//! `serde_json::Value` or any `Serialize` type are still available for
//! escape-hatch usage and backward compatibility.

use super::misc::TerraProvider;
use super::misc::TerraResource;
use beet_core::prelude::*;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use std::io::Write;
use std::path::Path;

/// A required provider declaration.
pub struct ProviderRequirement {
	/// The provider source, e.g. "hashicorp/aws"
	pub source: String,
	/// The version constraint, e.g. "~> 5.0"
	pub version: String,
}

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
/// # Example (typed)
/// ```rust,ignore
/// let bucket = AwsS3BucketDetails { bucket: Some("my-bucket".into()), ..Default::default() };
/// let exporter = ConfigExporter::new()
///     .with_resource("assets", &bucket)
///     .with_output("bucket_name", Output {
///         value: json!("${aws_s3_bucket.assets.bucket}"),
///         description: Some("The bucket name".into()),
///         sensitive: None,
///     });
/// exporter.export_to_file("main.tf.json").await?;
/// exporter.validate().await?;
/// ```
///
/// # Example (untyped / legacy)
/// ```rust,ignore
/// let mut exporter = ConfigExporter::new();
/// exporter.add_required_provider("aws", "hashicorp/aws", "~> 6.0");
/// exporter.add_untyped_provider("aws", &serde_json::json!({"region": "us-west-2"}))?;
/// exporter.add_untyped_resource("aws_instance", "web", &my_instance)?;
/// exporter.export_to_file("main.tf.json").await?;
/// ```
pub struct ConfigExporter {
	required_providers: Map<String, Value>,
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
	// Typed API — providers are inferred from resources
	// =====================================================================

	/// Add a typed resource. The provider requirement is registered
	/// automatically from the resource's [`TerraResource`] implementation.
	///
	/// Returns `self` for chaining.
	pub fn with_resource(
		mut self,
		name: impl Into<String>,
		resource: &dyn TerraResource,
	) -> Self {
		let provider = resource.provider();
		self.ensure_provider(provider);

		let resource_type = resource.resource_type().to_string();
		let value = resource.to_json();

		let type_map = self
			.resources
			.entry(resource_type)
			.or_insert_with(|| Value::Object(Map::new()));
		if let Value::Object(map) = type_map {
			map.insert(name.into(), value);
		}
		self
	}

	/// Add a typed resource via mutable reference (non-chaining variant).
	pub fn add_resource(
		&mut self,
		name: impl Into<String>,
		resource: &dyn TerraResource,
	) -> &mut Self {
		let provider = resource.provider();
		self.ensure_provider(provider);

		let resource_type = resource.resource_type().to_string();
		let value = resource.to_json();

		let type_map = self
			.resources
			.entry(resource_type)
			.or_insert_with(|| Value::Object(Map::new()));
		if let Value::Object(map) = type_map {
			map.insert(name.into(), value);
		}
		self
	}

	/// Add a provider configuration block for a provider that has already been
	/// auto-registered (or will be) by adding typed resources.
	pub fn with_provider_config(
		mut self,
		provider: &TerraProvider,
		config: &impl Serialize,
	) -> Result<Self> {
		self.ensure_provider(provider);
		let value = serde_json::to_value(config)?;
		self.providers
			.insert(provider.local_name().to_string(), value);
		Ok(self)
	}

	/// Add a variable definition (chaining).
	pub fn with_variable(
		mut self,
		name: impl Into<String>,
		variable: Variable,
	) -> Self {
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

	/// Add a local value (chaining).
	pub fn with_local(
		mut self,
		name: impl Into<String>,
		value: impl Serialize,
	) -> Result<Self> {
		let val = serde_json::to_value(value)?;
		self.locals.insert(name.into(), val);
		Ok(self)
	}

	/// Automatically register a provider's `required_providers` entry if it
	/// hasn't been registered yet.
	fn ensure_provider(&mut self, provider: &TerraProvider) {
		let local = provider.local_name().to_string();
		if self.required_providers.contains_key(&local) {
			return;
		}
		self.required_providers.insert(
			local,
			json!({
				"source": provider.short_source(),
				"version": provider.version.as_ref(),
			}),
		);
	}

	// =====================================================================
	// Untyped / legacy API
	// =====================================================================

	/// Add a required provider declaration.
	pub fn add_required_provider(
		&mut self,
		name: &str,
		source: &str,
		version: &str,
	) -> &mut Self {
		self.required_providers.insert(
			name.to_string(),
			json!({
				"source": source,
				"version": version,
			}),
		);
		self
	}

	/// Add a provider configuration block.
	pub fn add_untyped_provider(
		&mut self,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self, serde_json::Error> {
		let value = serde_json::to_value(config)?;
		self.providers.insert(name.to_string(), value);
		Ok(self)
	}

	/// Add a resource block.
	pub fn add_untyped_resource(
		&mut self,
		resource_type: &str,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self, serde_json::Error> {
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

	/// Add a data source block.
	pub fn add_data_source(
		&mut self,
		data_type: &str,
		name: &str,
		config: &impl Serialize,
	) -> Result<&mut Self, serde_json::Error> {
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

	/// Add a variable definition.
	pub fn add_variable(
		&mut self,
		name: &str,
		variable: Variable,
	) -> &mut Self {
		self.insert_variable(name, variable);
		self
	}

	/// Add an output definition.
	pub fn add_output(&mut self, name: &str, output: Output) -> &mut Self {
		self.insert_output(name, output);
		self
	}

	/// Add a local value.
	pub fn add_local(
		&mut self,
		name: &str,
		value: impl Serialize,
	) -> Result<&mut Self, serde_json::Error> {
		let val = serde_json::to_value(value)?;
		self.locals.insert(name.to_string(), val);
		Ok(self)
	}

	// =====================================================================
	// Serialization
	// =====================================================================

	/// Build the complete Terraform JSON configuration.
	pub fn to_value(&self) -> Value {
		let mut root = Map::new();

		// terraform block with required_providers
		if !self.required_providers.is_empty() {
			root.insert(
				"terraform".to_string(),
				json!({
					"required_providers": Value::Object(self.required_providers.clone()),
				}),
			);
		}

		// provider block
		if !self.providers.is_empty() {
			root.insert(
				"provider".to_string(),
				Value::Object(self.providers.clone()),
			);
		}

		// variable block
		if !self.variables.is_empty() {
			root.insert(
				"variable".to_string(),
				Value::Object(self.variables.clone()),
			);
		}

		// locals block
		if !self.locals.is_empty() {
			root.insert(
				"locals".to_string(),
				Value::Object(self.locals.clone()),
			);
		}

		// resource block
		if !self.resources.is_empty() {
			root.insert(
				"resource".to_string(),
				Value::Object(self.resources.clone()),
			);
		}

		// data block
		if !self.data_sources.is_empty() {
			root.insert(
				"data".to_string(),
				Value::Object(self.data_sources.clone()),
			);
		}

		// output block
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
	/// Returns `Ok(validate_json_output)` on success, or an error if
	/// `tofu init` or `tofu validate` fails.
	pub async fn validate(&self) -> Result<String> {
		let dir = TempDir::new()?;

		// Write the config
		let config_path = dir.as_ref().join("main.tf.json");
		self.export_to_file(&config_path).await?;

		// tofu init
		let init = async_process::Command::new("tofu")
			.current_dir(dir.as_ref())
			.args(["init"])
			.output()
			.await?;
		if !init.status.success() {
			let stderr = String::from_utf8_lossy(&init.stderr);
			bevybail!("tofu init failed:\n{}", stderr);
		}

		// tofu validate -json
		let validate = async_process::Command::new("tofu")
			.current_dir(dir.as_ref())
			.args(["validate", "-json"])
			.output()
			.await?;

		let stdout = String::from_utf8_lossy(&validate.stdout).to_string();

		if !validate.status.success() {
			let stderr = String::from_utf8_lossy(&validate.stderr);
			bevybail!(
				"tofu validate failed:\nstdout: {}\nstderr: {}",
				stdout,
				stderr
			);
		}

		// dir auto-cleans on drop
		Ok(stdout)
	}

	/// Export to `path`, then run `tofu init` + `tofu validate` in the
	/// directory containing the file. Prints progress via [`cross_log!`].
	///
	/// This is a convenience wrapper combining [`export_to_file`] and
	/// [`validate`] that operates in-place rather than using a temp dir.
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
			let stderr = String::from_utf8_lossy(&init.stderr);
			bevybail!("tofu init failed:\n{}", stderr);
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
			let stderr = String::from_utf8_lossy(&validate.stderr);
			bevybail!(
				"tofu validate failed:\nstdout: {}\nstderr: {}",
				stdout,
				stderr
			);
		}
		cross_log!("tofu validate: PASSED");

		Ok(stdout)
	}

	// =====================================================================
	// Internal helpers
	// =====================================================================

	fn insert_variable(&mut self, name: impl Into<String>, variable: Variable) {
		let mut var_obj = Map::new();
		if let Some(var_type) = variable.r#type {
			var_obj.insert("type".to_string(), Value::String(var_type));
		}
		if let Some(default) = variable.default {
			var_obj.insert("default".to_string(), default);
		}
		if let Some(desc) = variable.description {
			var_obj.insert("description".to_string(), Value::String(desc));
		}
		self.variables.insert(name.into(), Value::Object(var_obj));
	}

	fn insert_output(&mut self, name: impl Into<String>, output: Output) {
		let mut out_obj = Map::new();
		out_obj.insert("value".to_string(), output.value);
		if let Some(desc) = output.description {
			out_obj.insert("description".to_string(), Value::String(desc));
		}
		if let Some(sensitive) = output.sensitive {
			out_obj.insert("sensitive".to_string(), Value::Bool(sensitive));
		}
		self.outputs.insert(name.into(), Value::Object(out_obj));
	}
}

impl Default for ConfigExporter {
	fn default() -> Self { Self::new() }
}
