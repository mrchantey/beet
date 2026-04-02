use crate::prelude::*;
use std::collections::BTreeMap;

/// Code generation options meant to be supported by all languages.
#[derive(Clone, Debug)]
pub struct CodeGeneratorConfig {
	pub(crate) module_name: Option<String>,
	pub(crate) external_definitions: ExternalDefinitions,
	pub(crate) comments: DocComments,
	/// Use statements emitted in the file preamble, after the `#![allow(…)]`
	/// line.  Defaults include `BTreeMap as Map`, `serde`, and `serde_bytes`.
	/// The emitter always appends `beet_core::prelude::*` and
	/// `beet_infra::prelude::*` (resolved via [`pkg_ext`]) after these.
	/// Use [`with_use_items`](Self::with_use_items) to replace entirely, or
	/// [`with_additional_use_items`](Self::with_additional_use_items) to extend.
	pub(crate) use_items: Vec<String>,
	/// When `true`, convert all generated type names from `snake_case` to
	/// `UpperCamelCase` using the `heck` crate.
	pub(crate) use_title_case: bool,
	/// When `true`, emit the root enum types (`resource_root`, `provider_root`,
	/// `datasource_root`) and the top-level `config` struct.  When `false`
	/// (the default for filtered / typed generation), only resource detail
	/// structs and their block-type children are emitted.
	pub(crate) generate_roots: bool,
	/// Optional custom preamble to replace the default `#![allow(…)]` /
	/// `use serde::…` block at the top of the generated file.
	pub(crate) custom_preamble: Option<String>,
	/// When `true`, always derive `Default` for structs — even those with
	/// required (non-`Option`) fields.  This sacrifices compile-time safety
	/// for ergonomics (`..Default::default()` patterns).
	pub(crate) generate_default: bool,
	/// When `true`, emit `pub fn new(…)` constructors for structs that have
	/// at least one required field.  Optional fields are initialised to their
	/// natural defaults (`None`, `Vec::new()`, etc.).
	pub(crate) generate_builders: bool,
	/// When `true`, emit `TerraResource` and `TerraJson` trait
	/// implementations for each resource struct listed in `resource_meta`.
	pub(crate) generate_trait_impls: bool,
	/// Metadata about generated resource types — used to emit trait impls
	/// and builder constructors with the correct resource-type / provider
	/// information.
	pub(crate) resource_meta: Vec<ResourceMeta>,
}

/// Track types definitions provided by external modules.
pub type ExternalDefinitions = std::collections::BTreeMap<
	/* module */ String,
	/* type names */ Vec<String>,
>;

/// Track documentation to be attached to particular definitions.
pub type DocComments = std::collections::BTreeMap<
	/* qualified name */ Vec<String>,
	/* comment */ String,
>;

impl Default for CodeGeneratorConfig {
	fn default() -> Self { Self::new() }
}

impl CodeGeneratorConfig {
	/// Default use items included in the generated preamble.
	///
	/// Includes `BTreeMap as Map`, `serde`, and `serde_bytes`.
	pub fn default_use_items() -> Vec<String> {
		vec![
			"use std::collections::BTreeMap as Map;".into(),
			"use serde::{Serialize, Deserialize};".into(),
			"use serde_bytes::ByteBuf as Bytes;".into(),
		]
	}

	/// Default config.
	pub fn new() -> Self {
		Self {
			module_name: None,
			external_definitions: BTreeMap::new(),
			comments: BTreeMap::new(),
			use_items: Self::default_use_items(),
			use_title_case: false,
			generate_roots: true,
			custom_preamble: None,
			generate_default: true,
			generate_builders: true,
			generate_trait_impls: false,
			resource_meta: Vec::new(),
		}
	}

	/// Set the module name for code generation.
	pub fn with_module_name(mut self, name: impl Into<String>) -> Self {
		self.module_name = Some(name.into());
		self
	}

	/// Return the module name as a `&str`, falling back to `"default"`.
	pub(crate) fn module_name_str(&self) -> &str {
		self.module_name.as_deref().unwrap_or("default")
	}

	/// Container names provided by external modules.
	pub fn with_external_definitions(
		mut self,
		external_definitions: ExternalDefinitions,
	) -> Self {
		self.external_definitions = external_definitions;
		self
	}

	/// Comments attached to a particular entity.
	pub fn with_comments(mut self, mut comments: DocComments) -> Self {
		// Make sure comments end with a (single) newline.
		for comment in comments.values_mut() {
			*comment = format!("{}\n", comment.trim());
		}
		self.comments = comments;
		self
	}

	/// Enable or disable `UpperCamelCase` conversion for generated type names.
	pub fn with_title_case(mut self, enabled: bool) -> Self {
		self.use_title_case = enabled;
		self
	}

	/// Enable or disable generation of root enum / config types.
	pub fn with_generate_roots(mut self, enabled: bool) -> Self {
		self.generate_roots = enabled;
		self
	}

	/// Replace the default preamble (`#![allow(…)]`, `use serde::…`, etc.)
	/// with a custom one.  When `None`, the emitter writes its built-in
	/// preamble.
	pub fn with_custom_preamble(mut self, preamble: impl Into<String>) -> Self {
		self.custom_preamble = Some(preamble.into());
		self
	}

	/// Replace all use items in the generated preamble.
	///
	/// The emitter still appends the beet glob imports after these.
	pub fn with_use_items(mut self, items: Vec<String>) -> Self {
		self.use_items = items;
		self
	}

	/// Append additional use items to the preamble.
	pub fn with_additional_use_items(
		mut self,
		items: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		self.use_items.extend(items.into_iter().map(Into::into));
		self
	}

	/// When `true`, **always** derive `Default` for structs, even those with
	/// required fields.  This lets callers use `..Default::default()` at the
	/// cost of potentially constructing invalid resources.
	pub fn with_generate_default(mut self, enabled: bool) -> Self {
		self.generate_default = enabled;
		self
	}

	/// Enable or disable `pub fn new(…)` constructor generation for structs
	/// with required fields.
	pub fn with_generate_builders(mut self, enabled: bool) -> Self {
		self.generate_builders = enabled;
		self
	}

	/// Enable or disable `TerraResource` / `TerraJson` trait impl generation.
	/// Requires [`resource_meta`](Self::with_resource_meta) to be populated.
	pub fn with_generate_trait_impls(mut self, enabled: bool) -> Self {
		self.generate_trait_impls = enabled;
		self
	}

	/// Provide metadata about generated resource types so the emitter can
	/// produce trait implementations and correctly-typed constructors.
	pub fn with_resource_meta(mut self, meta: Vec<ResourceMeta>) -> Self {
		self.resource_meta = meta;
		self
	}
}
