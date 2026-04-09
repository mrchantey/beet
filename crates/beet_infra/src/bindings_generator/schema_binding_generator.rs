//! Roundtrip schema + binding generator.
//!
//! [`SchemaBindingGenerator`] orchestrates the full workflow:
//!
//! 1. Write a `providers.tf.json` declaring the required providers.
//! 2. Run `tofu init` to download provider plugins.
//! 3. Run `tofu providers schema -json` to export the full schema.
//! 4. Parse the schema with [`BindingGenerator`] (applying filters).
//! 5. Write the generated Rust files to the specified output paths.

use super::binding_generator::BindingGenerator;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ProviderBindingTarget — per-provider output configuration
// ---------------------------------------------------------------------------

/// Pairs a [`terra::Provider`] with a list of resource type names to generate.
///
/// This is a configuration struct — it does not generate anything on its own.
/// Pass it to [`BindingFile::with_resources`] to register which
/// provider resources should be generated.
pub struct ResourceList {
	/// The provider to generate bindings for.
	pub provider: terra::Provider,
	pub resources: Vec<String>,
}

impl ResourceList {
	pub fn new(provider: terra::Provider, resources: Vec<String>) -> Self {
		Self {
			provider,
			resources,
		}
	}
}

// ---------------------------------------------------------------------------
// SchemaBindingGenerator
// ---------------------------------------------------------------------------

/// Orchestrates the full roundtrip: providers → tofu init → schema → codegen.
///
/// Holds a [`BindingGenerator`] that can be customised before generation.
/// The binding generator's [`CodeGeneratorConfig`] controls all code-generation
/// options (title case, builders, trait impls, preamble, etc.).
///
/// # Example
///
/// ```rust,ignore
/// SchemaBindingGenerator::default()
///     .with_file(
///         BindingFile::new("src/providers/aws_lambda.rs")
///             .with_resources(terra::Provider::AWS, ["aws_lambda_function", "aws_s3_bucket"]),
///     )
///     .with_file(
///         BindingFile::new("src/providers/cloudflare_dns.rs")
///             .with_resources(terra::Provider::CLOUDFLARE, ["cloudflare_dns_record"]),
///     )
///     .generate()
///     .await?;
/// ```
pub struct SchemaBindingGenerator {
	/// Each entry maps a provider binding target to its list of resource type names.
	files: Vec<BindingFile>,
	/// Working directory for tofu operations.  Defaults to
	/// `target/terra-bindings-generator`.
	work_dir: AbsPathBuf,
	/// The binding generator used for each target.  Users can pre-configure
	/// this to control code-generation options; per-target filter and preamble
	/// are applied automatically on top.
	binding_generator: BindingGenerator,
}

/// A single output file with one or more provider resource lists.
pub struct BindingFile {
	/// Destination file path (relative to the crate root), e.g.
	/// `"src/providers/aws_lambda.rs"`.
	pub path: PathBuf,
	resources: Vec<ResourceList>,
}

impl BindingFile {
	pub fn new(path: impl AsRef<Path>) -> Self {
		Self {
			path: path.as_ref().to_path_buf(),
			resources: Vec::new(),
		}
	}

	pub fn with_resources(
		mut self,
		provider: terra::Provider,
		resources: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		self.resources.push(ResourceList::new(
			provider,
			resources.into_iter().map(Into::into).collect(),
		));
		self
	}
}

impl Default for SchemaBindingGenerator {
	fn default() -> Self {
		Self {
			files: Vec::new(),
			work_dir: WsPathBuf::new("target/terra-bindings-generator")
				.into_abs(),
			binding_generator: BindingGenerator::new()
				.with_title_case(true)
				.with_trait_impls(true)
				.with_custom_preamble(build_preamble()),
		}
	}
}

impl SchemaBindingGenerator {
	/// Add a provider and its resource list.
	pub fn with_file(mut self, file: BindingFile) -> Self {
		self.files.push(file);
		self
	}

	/// Override the working directory used for `tofu init` / schema export.
	pub fn with_work_dir(mut self, dir: AbsPathBuf) -> Self {
		self.work_dir = dir;
		self
	}

	/// Replace the [`BindingGenerator`] used for code generation.
	///
	/// The filter and custom preamble are still set per-target automatically;
	/// everything else (title case, builders, trait impls, etc.) comes from
	/// the generator you supply here.
	pub fn with_binding_generator(
		mut self,
		generator: BindingGenerator,
	) -> Self {
		self.binding_generator = generator;
		self
	}

	/// Return a shared reference to the current [`BindingGenerator`].
	pub fn binding_generator(&self) -> &BindingGenerator {
		&self.binding_generator
	}

	/// Return a mutable reference to the current [`BindingGenerator`].
	pub fn binding_generator_mut(&mut self) -> &mut BindingGenerator {
		&mut self.binding_generator
	}

	/// Run the full generation pipeline.
	///
	/// Caches `providers.tf.json` and reuses `schema.json` when the provider
	/// configuration has not changed, skipping the slow `tofu init` and
	/// `tofu providers schema` steps.
	pub async fn generate(&self) -> Result {
		let new_content = self.build_providers_tf_content()?;
		let providers_path = self.work_dir.join("providers.tf.json");
		let schema_path = self.work_dir.join("schema.json");

		let can_reuse = providers_path.exists()
			&& schema_path.exists()
			&& fs_ext::read(&providers_path)
				.map(|existing| existing == new_content)
				.unwrap_or(false);

		if can_reuse {
			cross_log!(
				"[schema_binding_generator] providers unchanged, reusing existing schema"
			);
		} else {
			// 1. Prepare the working directory.
			self.prepare_work_dir()?;

			// 2. Write providers.tf.json
			self.write_providers_tf_bytes(&new_content)?;

			// 3. tofu init
			self.run_tofu_init().await?;

			// 4. tofu providers schema -json > schema.json
			self.run_tofu_schema().await?;
		}

		// 5. For each provider target, generate bindings with appropriate filter.
		self.generate_bindings(&schema_path)?;

		Ok(())
	}

	/// Like [`generate`](Self::generate) but skip steps 1–4 and use an
	/// existing `schema.json` file directly.  Useful when the schema has
	/// already been exported (saves the slow `tofu init` step).
	pub fn generate_from_schema(
		&self,
		schema_path: impl AsRef<Path>,
	) -> Result {
		self.generate_bindings(schema_path.as_ref())
	}

	// ------------------------------------------------------------------
	// Internal steps
	// ------------------------------------------------------------------

	fn prepare_work_dir(&self) -> Result {
		if self.work_dir.exists() {
			fs_ext::remove(&self.work_dir)?;
		}
		fs_ext::create_dir_all(&self.work_dir)?;
		Ok(())
	}

	/// Build the serialized `providers.tf.json` content as bytes.
	fn build_providers_tf_content(&self) -> Result<Vec<u8>> {
		let mut required_providers = serde_json::Map::new();

		for file in &self.files {
			for list in &file.resources {
				// Deduplicate by local name.
				let local = list.provider.local_name().to_string();
				if required_providers.contains_key(&local) {
					continue;
				}
				required_providers.insert(
					local,
					json!({
						"source": list.provider.short_source(),
						"version": list.provider.version.as_ref(),
					}),
				);
			}
		}

		let tf_json = json!({
			"terraform": {
				"required_providers": required_providers,
			}
		});

		let mut buf = Vec::new();
		serde_json::to_writer_pretty(&mut buf, &tf_json)?;
		buf.write_all(b"\n")?;
		Ok(buf)
	}

	/// Write pre-built providers content to `providers.tf.json`.
	fn write_providers_tf_bytes(&self, content: &[u8]) -> Result {
		let path = self.work_dir.join("providers.tf.json");
		fs_ext::write(&path, content)?;
		cross_log!("[schema_binding_generator] wrote {}", path.display());
		Ok(())
	}

	async fn run_tofu_init(&self) -> Result {
		cross_log!(
			"[schema_binding_generator] running tofu init in {}",
			self.work_dir.display()
		);
		tofu::init(&self.work_dir, false).await?;

		cross_log!("[schema_binding_generator] tofu init: OK");
		Ok(())
	}

	async fn run_tofu_schema(&self) -> Result<AbsPathBuf> {
		let schema_path = self.work_dir.join("schema.json");
		cross_log!(
			"[schema_binding_generator] running tofu providers schema → {}",
			schema_path
		);
		let schema = tofu::export_schema(&self.work_dir).await?;

		fs_ext::write_async(&schema_path, &schema).await?;

		cross_log!(
			"[schema_binding_generator] schema exported ({:.1} MB)",
			schema.len() as f64 / 1_048_576.0
		);
		Ok(schema_path)
	}

	fn generate_bindings(&self, schema_path: &Path) -> Result {
		let schema = BindingGenerator::read_schema(schema_path)?;

		for file in &self.files {
			let mut filter = terra::ResourceFilter::default();
			for list in &file.resources {
				filter = filter.with_resources(
					list.provider.source.as_ref(),
					&list.resources,
				);
			}

			// Clone the base binding generator and apply the per-target filter.
			let binding_gen =
				self.binding_generator.clone().with_filter(filter);

			// Ensure the parent directory exists.
			if let Some(parent) = file.path.parent() {
				fs_ext::create_dir_all(parent)?;
			}

			binding_gen.generate_to_file(&schema, &file.path)?;
			cross_log!(
				"[schema_binding_generator] wrote {}",
				file.path.display()
			);
		}

		Ok(())
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Standard preamble for generated provider modules.
fn build_preamble() -> String {
	[
		"//! Auto-generated Terraform provider bindings — do not edit!",
		"//! Auto-generated Terraform provider bindings — do not edit!",
		"//! Auto-generated Terraform provider bindings — do not edit!",
		"",
		"#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]",
		"use std::collections::BTreeMap as Map;",
		"use serde::{Serialize, Deserialize};",
		"use serde_json;",
		"#[allow(unused)]",
		"use beet_core::prelude::*;",
		"#[allow(unused)]",
		"use crate::prelude::*;",
	]
	.join("\n")
}
