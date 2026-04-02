//! High-level binding generator with configurable options.
//!
//! Wraps the lower-level `binding` functions and provides a convenient
//! API for reading schemas, configuring output, and writing generated Rust
//! bindings to various destinations.
//!
//! Generation options (title-case, builders, trait impls, custom preamble,
//! etc.) are configured via [`CodeGeneratorConfig`] which is held as a field.
//! Use [`with_code_generator_config`](BindingGenerator::with_code_generator_config)
//! to supply a fully customised config, or the convenience `with_*` methods
//! which forward to the underlying config.

use super::binding::TerraformSchemaExport;
use super::binding::export_filtered_resources;
use super::binding::export_schema_to_registry;
use super::binding::read_tf_schema_from_file;
use super::config::CodeGeneratorConfig;
use super::emit::CodeGenerator;
use super::ir::Registry;
use crate::prelude::*;
use beet_core::prelude::*;
use std::io::Write;
use std::path::Path;

/// High-level options for Terraform schema binding generation.
///
/// # Example — filtered, typed generation
///
/// ```rust,ignore
/// let filter = ResourceFilter::default()
///     .with_resources("registry.opentofu.org/hashicorp/aws", [
///         "aws_lambda_function",
///         "aws_s3_bucket",
///     ]);
///
/// let schema = BindingGenerator::read_schema("schema.json")?;
/// let generator = BindingGenerator::new()
///     .with_filter(filter)
///     .with_title_case(true);
///
/// generator.generate_to_file(&schema, "src/providers/aws_lambda.rs")?;
/// ```
#[derive(Clone)]
pub struct BindingGenerator {
	/// All code-generation options (title case, builders, trait impls,
	/// preamble, etc.) live here.
	config: CodeGeneratorConfig,

	/// Optional resource filter.  When set, only the specified resources are
	/// parsed from the schema.
	filter: Option<ResourceFilter>,
}

impl Default for BindingGenerator {
	fn default() -> Self {
		Self {
			config: CodeGeneratorConfig::default(),
			filter: None,
		}
	}
}

impl BindingGenerator {
	/// Create a new `BindingGenerator` with default options.
	pub fn new() -> Self { Self::default() }

	// ------------------------------------------------------------------
	// Configuration — direct config access
	// ------------------------------------------------------------------

	/// Replace the entire [`CodeGeneratorConfig`].
	///
	/// Use this when you need full control over every code-generation knob
	/// rather than calling the individual `with_*` convenience methods.
	pub fn with_code_generator_config(
		mut self,
		config: CodeGeneratorConfig,
	) -> Self {
		self.config = config;
		self
	}

	/// Return a shared reference to the current [`CodeGeneratorConfig`].
	pub fn code_generator_config(&self) -> &CodeGeneratorConfig { &self.config }

	/// Return a mutable reference to the current [`CodeGeneratorConfig`].
	pub fn code_generator_config_mut(&mut self) -> &mut CodeGeneratorConfig {
		&mut self.config
	}

	// ------------------------------------------------------------------
	// Configuration — convenience forwards
	// ------------------------------------------------------------------

	/// Enable or disable `new()` constructor generation.
	pub fn with_builders(mut self, enabled: bool) -> Self {
		self.config = self.config.with_generate_builders(enabled);
		self
	}

	/// Enable or disable `UpperCamelCase` type-name conversion.
	pub fn with_title_case(mut self, enabled: bool) -> Self {
		self.config = self.config.with_title_case(enabled);
		self
	}

	/// Set the resource filter.
	pub fn with_filter(mut self, filter: ResourceFilter) -> Self {
		self.filter = Some(filter);
		self
	}

	/// Enable generation of `TerraResource` / `TerraJson` trait impls.
	pub fn with_trait_impls(mut self, enabled: bool) -> Self {
		self.config = self.config.with_generate_trait_impls(enabled);
		self
	}

	/// Replace the default preamble (`#![allow(…)]`, `use serde::…`, etc.)
	/// with a custom one.
	pub fn with_custom_preamble(mut self, preamble: impl Into<String>) -> Self {
		self.config = self.config.with_custom_preamble(preamble);
		self
	}

	/// When `true`, always derive `Default` for structs — even those with
	/// required fields.
	pub fn with_generate_default(mut self, enabled: bool) -> Self {
		self.config = self.config.with_generate_default(enabled);
		self
	}

	// ------------------------------------------------------------------
	// Schema I/O
	// ------------------------------------------------------------------

	/// Read a Terraform provider schema from a JSON file on disk.
	pub fn read_schema(
		path: impl AsRef<Path>,
	) -> Result<TerraformSchemaExport> {
		read_tf_schema_from_file(path)
	}

	/// Read a schema file and return a default generator together with the
	/// parsed schema — a convenience shorthand for the common case.
	pub fn from_schema_file(
		path: impl AsRef<Path>,
	) -> Result<(Self, TerraformSchemaExport)> {
		let schema = read_tf_schema_from_file(path)?;
		Ok((Self::default(), schema))
	}

	// ------------------------------------------------------------------
	// Code generation
	// ------------------------------------------------------------------

	/// Generate Rust bindings for the given schema and write them to `out`.
	pub fn generate_to_writer(
		&self,
		schema: &TerraformSchemaExport,
		out: &mut dyn Write,
	) -> Result {
		let config = self.build_config(schema)?;
		let registry = self.build_registry(schema)?;

		CodeGenerator::new(&config).output(out, &registry)?;
		Ok(())
	}

	/// Generate Rust bindings and return them as a `String`.
	pub fn generate_to_string(
		&self,
		schema: &TerraformSchemaExport,
	) -> Result<String> {
		let mut buf = Vec::new();
		self.generate_to_writer(schema, &mut buf)?;
		Ok(String::from_utf8(buf)?)
	}

	/// Generate Rust bindings and write them to a file at `path`.
	pub fn generate_to_file(
		&self,
		schema: &TerraformSchemaExport,
		path: impl AsRef<Path>,
	) -> Result {
		let output = self.generate_to_string(schema)?;
		fs_ext::write(path, output)?;
		Ok(())
	}

	// ------------------------------------------------------------------
	// Internal helpers
	// ------------------------------------------------------------------

	/// Build the final [`CodeGeneratorConfig`] by merging the stored config
	/// with data extracted from the schema (resource meta, doc comments,
	/// root-generation flag).
	fn build_config(
		&self,
		schema: &TerraformSchemaExport,
	) -> Result<CodeGeneratorConfig> {
		let mut config = self.config.clone();

		if let Some(filter) = &self.filter {
			let (_registry, meta, comments) =
				export_filtered_resources(schema, filter, &config)?;
			config = config.with_resource_meta(meta).with_comments(comments);
		} else {
			// Unfiltered: full schema with root types.
			config = config.with_generate_roots(true);
		}

		Ok(config)
	}

	/// Build the serde-reflection registry, using filtering when configured.
	fn build_registry(
		&self,
		schema: &TerraformSchemaExport,
	) -> Result<Registry> {
		if let Some(filter) = &self.filter {
			let (registry, _meta, _comments) =
				export_filtered_resources(schema, filter, &self.config)?;
			Ok(registry)
		} else {
			let registry = export_schema_to_registry(schema)?;
			Ok(registry)
		}
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use crate::bindings_generator::*;
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn default_generator() {
		let generator = BindingGenerator::default();
		generator.filter.is_none().xpect_true();
	}

	#[test]
	fn new_equals_default() {
		let gen_a = BindingGenerator::new();
		let gen_b = BindingGenerator::default();
		gen_a
			.config
			.use_title_case
			.xpect_eq(gen_b.config.use_title_case);
		gen_a
			.config
			.generate_builders
			.xpect_eq(gen_b.config.generate_builders);
	}

	#[test]
	fn with_code_generator_config() {
		let config = CodeGeneratorConfig::new()
			.with_module_name("custom")
			.with_title_case(true)
			.with_generate_builders(false);

		let generator =
			BindingGenerator::new().with_code_generator_config(config);
		generator.config.use_title_case.xpect_true();
		generator.config.generate_builders.xpect_false();
	}

	#[test]
	fn convenience_forwards() {
		let generator = BindingGenerator::new()
			.with_title_case(true)
			.with_builders(false)
			.with_trait_impls(true)
			.with_generate_default(true);

		generator.config.use_title_case.xpect_true();
		generator.config.generate_builders.xpect_false();
		generator.config.generate_trait_impls.xpect_true();
		generator.config.generate_default.xpect_true();
	}

	#[test]
	fn format_helpers_accessible_via_emit() {
		// Verify the IR module's helpers work.
		FieldType::Option(Box::new(FieldType::Str))
			.is_optional()
			.xpect_true();
	}

	#[test]
	fn generate_builder_impls_skips_all_optional() {
		let config = CodeGeneratorConfig::new().with_generate_builders(true);

		let mut registry = Registry::new();
		registry.insert(
			(None, "AllOptional".to_string()),
			Container::Struct(vec![
				Field::new("a", FieldType::Option(Box::new(FieldType::Str))),
				Field::new("b", FieldType::Option(Box::new(FieldType::Bool))),
			]),
		);

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		// All-optional struct should NOT have a builder impl.
		output.contains("impl AllOptional {").xpect_false();
	}

	#[test]
	fn generate_builder_impls_with_required() {
		let config = CodeGeneratorConfig::new().with_generate_builders(true);

		let mut registry = Registry::new();
		registry.insert(
			(None, "MyStruct".to_string()),
			Container::Struct(vec![
				Field::new("name", FieldType::Str),
				Field::new("count", FieldType::I64),
				Field::new(
					"label",
					FieldType::Option(Box::new(FieldType::Str)),
				),
			]),
		);

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		output.contains("impl MyStruct {").xpect_true();
		output
			.contains("pub fn new(name: SmolStr, count: i64) -> Self {")
			.xpect_true();
		output.contains("name").xpect_true();
		output.contains("count").xpect_true();
		output.contains("label: None").xpect_true();
	}

	#[test]
	fn generate_builder_impls_with_namespace() {
		let config = CodeGeneratorConfig::new().with_generate_builders(true);

		let mut registry = Registry::new();
		registry.insert(
			(Some("resource".to_string()), "my_thing".to_string()),
			Container::Struct(vec![Field::new("id", FieldType::Str)]),
		);

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		output.contains("impl resource_my_thing {").xpect_true();
	}

	#[test]
	fn generate_builder_impls_title_case() {
		let config = CodeGeneratorConfig::new()
			.with_title_case(true)
			.with_generate_builders(true);

		let mut registry = Registry::new();
		registry.insert(
			(None, "my_struct".to_string()),
			Container::Struct(vec![Field::new("id", FieldType::Str)]),
		);

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		output.contains("impl MyStruct {").xpect_true();
	}

	#[test]
	fn generate_terra_impls() {
		let meta = vec![ResourceMeta {
			resource_type: "aws_s3_bucket".to_string(),
			provider_source: "registry.opentofu.org/hashicorp/aws".to_string(),
			struct_name: "AwsS3BucketDetails".to_string(),
		}];

		let config = CodeGeneratorConfig::new()
			.with_title_case(true)
			.with_generate_trait_impls(true)
			.with_generate_builders(false)
			.with_resource_meta(meta);

		let mut registry = Registry::new();
		registry.insert(
			(None, "AwsS3BucketDetails".to_string()),
			Container::Struct(vec![Field::new(
				"bucket",
				FieldType::Option(Box::new(FieldType::Str)),
			)]),
		);

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		output
			.xpect_contains("impl TerraJson for AwsS3BucketDetails")
			.xpect_contains("impl TerraResource for AwsS3BucketDetails")
			.xpect_contains("\"aws_s3_bucket\"")
			.xpect_contains("TerraProvider::AWS");
	}

	#[test]
	fn custom_preamble() {
		let config = CodeGeneratorConfig::new()
			.with_custom_preamble("// custom preamble\nuse custom::stuff;")
			.with_generate_builders(false);

		let registry = Registry::new();

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		output.starts_with("// custom preamble").xpect_true();
		output.contains("use custom::stuff;").xpect_true();
		// Should NOT contain the default preamble.
		output
			.contains("use serde_bytes::ByteBuf as Bytes;")
			.xpect_false();
	}

	#[test]
	fn generate_default_forces_default_derive() {
		let config = CodeGeneratorConfig::new()
			.with_generate_default(true)
			.with_generate_builders(false);

		let mut registry = Registry::new();
		registry.insert(
			(None, "RequiredFields".to_string()),
			Container::Struct(vec![Field::new("name", FieldType::Str)]),
		);

		let mut buf = Vec::new();
		CodeGenerator::new(&config)
			.output(&mut buf, &registry)
			.unwrap();
		let output = String::from_utf8(buf).unwrap();

		output.contains("Default").xpect_true();
	}
}
