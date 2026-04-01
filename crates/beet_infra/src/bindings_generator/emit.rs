//! Token-based Rust code emitter for Terraform schema bindings.

use super::config::CodeGeneratorConfig;
use super::ir::Container;
use super::ir::Field;
use super::ir::FieldType;
use super::ir::Registry;
use super::ir::Variant;
use super::ir::VariantFormat;
use beet_core::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

/// Main configuration object for code-generation in Rust.
pub struct CodeGenerator<'a> {
	/// Language-independent configuration.
	config: &'a CodeGeneratorConfig,
	/// Which derive macros should be added (independently from serialization).
	derive_macros: Vec<String>,
	/// Additional block of text added before each new container definition.
	custom_derive_block: Option<String>,
	/// Whether definitions and fields should be marked as `pub`.
	track_visibility: bool,
}

/// Internal state carried through the emission of a single file.
struct EmitterState {
	/// When title-case is enabled, maps original full names to their
	/// `UpperCamelCase` equivalents so that type-name references can be
	/// rewritten consistently.
	type_renames: HashMap<String, String>,
	/// Track which definitions have a known (finite) size so far.
	/// Used to decide whether to `Box` a type reference.
	known_sizes: HashSet<String>,
}

impl<'a> CodeGenerator<'a> {
	/// Create a Rust code generator for the given config.
	pub fn new(config: &'a CodeGeneratorConfig) -> Self {
		Self {
			config,
			derive_macros: vec!["Clone", "Debug", "PartialEq", "PartialOrd"]
				.into_iter()
				.map(String::from)
				.collect(),
			custom_derive_block: None,
			track_visibility: true,
		}
	}

	/// Which derive macros should be added (independently from serialization).
	pub fn with_derive_macros(mut self, derive_macros: Vec<String>) -> Self {
		self.derive_macros = derive_macros;
		self
	}

	/// Additional block of text added after `derive_macros` (if any), before
	/// each new container definition.
	pub fn with_custom_derive_block(
		mut self,
		custom_derive_block: Option<String>,
	) -> Self {
		self.custom_derive_block = custom_derive_block;
		self
	}

	/// Whether definitions and fields should be marked as `pub`.
	pub fn with_track_visibility(mut self, track_visibility: bool) -> Self {
		self.track_visibility = track_visibility;
		self
	}

	/// Write container definitions in Rust.
	pub fn output(&self, out: &mut dyn Write, registry: &Registry) -> Result {
		let external_names: HashSet<String> = self
			.config
			.external_definitions
			.values()
			.cloned()
			.flatten()
			.collect();

		let known_sizes: HashSet<String> =
			external_names.iter().cloned().collect();

		let type_renames = if self.config.use_title_case {
			Self::build_rename_map(registry)
		} else {
			HashMap::new()
		};

		let mut state = EmitterState {
			type_renames,
			known_sizes,
		};

		// Preamble is emitted as raw text to preserve comments and inner
		// attributes that don't survive token round-tripping.
		let preamble = self.build_preamble_text(&external_names);

		// Collect all tokens for the file body.
		let mut file_tokens = TokenStream::new();

		for ((ns, name), format) in registry {
			file_tokens.extend(self.emit_container(&state, ns, name, format));

			if let Container::Struct(fields) = format {
				let struct_name = resolve_struct_name(&state, ns, name);

				if self.config.generate_builders {
					file_tokens.extend(self.emit_builder_impl(
						&state,
						&struct_name,
						fields,
					));
				}

				if self.config.generate_trait_impls {
					file_tokens.extend(self.emit_trait_impls(&struct_name));
				}
			}

			state.known_sizes.insert(name.clone());
		}

		// Write preamble as-is, then format body via prettyplease.
		out.write_all(preamble.as_bytes())?;
		if !file_tokens.is_empty() {
			let source = tokens_to_source(file_tokens)?;
			out.write_all(source.as_bytes())?;
		}
		Ok(())
	}

	// ------------------------------------------------------------------
	// Title-case helpers
	// ------------------------------------------------------------------

	fn build_rename_map(registry: &Registry) -> HashMap<String, String> {
		let mut map = HashMap::new();
		for (ns, name) in registry.keys() {
			let full_name = match ns {
				Some(namespace) => format!("{}_{}", namespace, name),
				None => name.clone(),
			};
			let title = full_name.to_upper_camel_case();
			if title != full_name {
				map.insert(full_name, title);
			}
		}
		map
	}

	// ------------------------------------------------------------------
	// Preamble
	// ------------------------------------------------------------------

	fn build_preamble_text(&self, external_names: &HashSet<String>) -> String {
		if let Some(preamble) = &self.config.custom_preamble {
			return format!("{}\n\n", preamble);
		}

		let mut lines = Vec::new();
		lines.push(
			"#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]"
				.to_string(),
		);
		if !external_names.contains("Map") {
			lines.push("use std::collections::BTreeMap as Map;".to_string());
		}
		lines.push("use serde::{Serialize, Deserialize};".to_string());
		if !external_names.contains("Bytes") {
			lines.push("use serde_bytes::ByteBuf as Bytes;".to_string());
		}
		for (module, definitions) in &self.config.external_definitions {
			if !module.is_empty() {
				lines.push(format!(
					"use {}::{{{}}};",
					module,
					definitions.join(", ")
				));
			}
		}
		lines.push(String::new());
		format!("{}\n", lines.join("\n"))
	}

	// ------------------------------------------------------------------
	// Container emission
	// ------------------------------------------------------------------

	fn emit_container(
		&self,
		state: &EmitterState,
		namespace: &Option<String>,
		name: &str,
		format: &Container,
	) -> TokenStream {
		let comment_tokens = self.emit_comment(name);
		let mut derive_macros = self.derive_macros.clone();
		derive_macros.push("Serialize".to_string());
		derive_macros.push("Deserialize".to_string());

		let vis = if self.track_visibility {
			quote! { pub }
		} else {
			quote! {}
		};

		match format {
			Container::UnitStruct => {
				let ident = Ident::new(name, Span::call_site());
				let derives = derive_idents(&derive_macros);
				let custom = self.custom_derive_tokens();
				quote! {
					#comment_tokens
					#[derive(#(#derives),*)]
					#custom
					struct #ident;
				}
			}
			Container::NewTypeStruct(fmt) => {
				let ident = Ident::new(name, Span::call_site());
				let derives = derive_idents(&derive_macros);
				let custom = self.custom_derive_tokens();
				let inner_ty = quote_field_type(state, fmt, true);
				quote! {
					#comment_tokens
					#[derive(#(#derives),*)]
					#custom
					struct #ident(#vis #inner_ty);
				}
			}
			Container::TupleStruct(formats) => {
				let ident = Ident::new(name, Span::call_site());
				let derives = derive_idents(&derive_macros);
				let custom = self.custom_derive_tokens();
				let types: Vec<_> = formats
					.iter()
					.map(|field_type| quote_field_type(state, field_type, true))
					.collect();
				quote! {
					#comment_tokens
					#[derive(#(#derives),*)]
					#custom
					struct #ident(#(#types),*);
				}
			}
			Container::Struct(fields) => {
				let all_optional =
					fields.iter().all(|field| field.value.is_optional());
				if all_optional || self.config.generate_default {
					derive_macros.push("Default".to_string());
				}
				let derives = derive_idents(&derive_macros);
				let custom = self.custom_derive_tokens();

				let (struct_ident, serde_rename) = if let Some(ns) = namespace {
					let full = format!("{}_{}", ns, name);
					let final_name = rename_type(state, &full);
					let ident = Ident::new(&final_name, Span::call_site());
					let rename_attr = quote! { #[serde(rename = #name)] };
					(ident, rename_attr)
				} else {
					let final_name = rename_type(state, name);
					let ident = Ident::new(&final_name, Span::call_site());
					(ident, quote! {})
				};

				let field_tokens: Vec<_> = fields
					.iter()
					.map(|field| {
						self.emit_struct_field(state, name, field, true)
					})
					.collect();

				quote! {
					#comment_tokens
					#[derive(#(#derives),*)]
					#custom
					#serde_rename
					#vis struct #struct_ident {
						#(#field_tokens)*
					}
				}
			}
			Container::Enum(variants) => {
				let derives = derive_idents(&derive_macros);
				let custom = self.custom_derive_tokens();
				let ident = Ident::new(name, Span::call_site());

				let variant_tokens: Vec<_> = variants
					.values()
					.map(|variant| self.emit_variant(state, name, variant))
					.collect();

				quote! {
					#comment_tokens
					#[derive(#(#derives),*)]
					#custom
					#vis enum #ident {
						#(#variant_tokens)*
					}
				}
			}
		}
	}

	// ------------------------------------------------------------------
	// Field & variant helpers
	// ------------------------------------------------------------------

	fn emit_struct_field(
		&self,
		state: &EmitterState,
		_container_name: &str,
		field: &Field,
		top_level: bool,
	) -> TokenStream {
		let vis = if top_level && self.track_visibility {
			quote! { pub }
		} else {
			quote! {}
		};

		let field_comment = self.emit_field_comment(&field.name);
		let serde_attr = field_serde_annotation(&field.value);
		let field_ident = make_field_ident(&field.name);
		let ty = quote_field_type(state, &field.value, true);

		quote! {
			#field_comment
			#serde_attr
			#vis #field_ident: #ty,
		}
	}

	fn emit_variant(
		&self,
		state: &EmitterState,
		_base: &str,
		variant: &Variant,
	) -> TokenStream {
		let variant_comment = self.emit_field_comment(&variant.name);
		let ident = Ident::new(&variant.name, Span::call_site());

		match &variant.value {
			VariantFormat::Unit => {
				quote! { #variant_comment #ident, }
			}
			VariantFormat::NewType(ft) => {
				let ty = quote_field_type(state, ft, true);
				quote! { #variant_comment #ident(#ty), }
			}
			VariantFormat::Tuple(fts) => {
				let types: Vec<_> = fts
					.iter()
					.map(|field_type| quote_field_type(state, field_type, true))
					.collect();
				quote! { #variant_comment #ident(#(#types),*), }
			}
			VariantFormat::Struct(fields) => {
				let field_tokens: Vec<_> = fields
					.iter()
					.map(|field| {
						let fi = make_field_ident(&field.name);
						let ty = quote_field_type(state, &field.value, true);
						quote! { #fi: #ty, }
					})
					.collect();
				quote! {
					#variant_comment
					#ident {
						#(#field_tokens)*
					},
				}
			}
		}
	}

	// ------------------------------------------------------------------
	// Comment helpers
	// ------------------------------------------------------------------

	fn emit_comment(&self, name: &str) -> TokenStream {
		let mut path = vec![self.config.module_name_str().to_string()];
		path.push(name.to_string());
		emit_doc_from_comments(&self.config.comments, &path)
	}

	fn emit_field_comment(&self, field_name: &str) -> TokenStream {
		// Field comments use a 3-element key: [module, container, field].
		// We don't have the container name in scope here, so we search for
		// any key ending with this field name.  The config's comment map is
		// keyed as [module, container, field], but the old emitter only used
		// [current_namespace..., field_name].  We replicate that by checking
		// all comments whose last element matches.
		//
		// In practice, comments are populated by `collect_descriptions` which
		// uses a full 3-element key.  We emit them from `emit_struct_field`
		// which doesn't track the container path.  Rather than thread it
		// through, we just look for any matching last-element — this is safe
		// because field names within a module are unique in practice.
		for (key, doc) in &self.config.comments {
			if key.last().map(|seg| seg.as_str()) == Some(field_name) {
				return emit_doc_string(doc);
			}
		}
		quote! {}
	}

	// ------------------------------------------------------------------
	// Builder (`new()`) generation
	// ------------------------------------------------------------------

	fn emit_builder_impl(
		&self,
		_state: &EmitterState,
		struct_name: &str,
		fields: &[Field],
	) -> TokenStream {
		let required: Vec<&Field> = fields
			.iter()
			.filter(|field| !field.value.is_optional())
			.collect();

		if required.is_empty() {
			return quote! {};
		}

		let struct_ident = Ident::new(struct_name, Span::call_site());

		let params: Vec<TokenStream> = required
			.iter()
			.map(|field| {
				let fi = make_field_ident(&field.name);
				let ty = format_to_type_tokens(
					&field.value,
					self.config.use_title_case,
				);
				quote! { #fi: #ty }
			})
			.collect();

		let inits: Vec<TokenStream> = fields
			.iter()
			.map(|field| {
				let fi = make_field_ident(&field.name);
				if field.value.is_optional() {
					let default = default_value_tokens(&field.value);
					quote! { #fi: #default }
				} else {
					quote! { #fi }
				}
			})
			.collect();

		quote! {
			impl #struct_ident {
				pub fn new(#(#params),*) -> Self {
					Self {
						#(#inits,)*
					}
				}
			}
		}
	}

	// ------------------------------------------------------------------
	// Trait impl generation
	// ------------------------------------------------------------------

	fn emit_trait_impls(&self, struct_name: &str) -> TokenStream {
		let meta = &self.config.resource_meta;
		let matching =
			meta.iter().find(|entry| entry.struct_name == struct_name);

		let entry = match matching {
			Some(entry) => entry,
			None => return quote! {},
		};

		let struct_ident = Ident::new(struct_name, Span::call_site());
		let resource_type_str = &entry.resource_type;
		let provider_const = provider_source_to_const(&entry.provider_source);
		let provider_ident = Ident::new(&provider_const, Span::call_site());

		quote! {
			impl crate::terra::TerraJson for #struct_ident {
				fn to_json(&self) -> serde_json::Value {
					serde_json::to_value(self).expect("serialization should not fail")
				}
			}

			impl crate::terra::TerraResource for #struct_ident {
				fn resource_type(&self) -> &'static str { #resource_type_str }
				fn provider(&self) -> &'static crate::terra::TerraProvider { &crate::terra::TerraProvider::#provider_ident }
			}
		}
	}

	// ------------------------------------------------------------------
	// Misc helpers
	// ------------------------------------------------------------------

	fn custom_derive_tokens(&self) -> TokenStream {
		match &self.custom_derive_block {
			Some(text) => text.parse().unwrap_or_default(),
			None => quote! {},
		}
	}
}

// =========================================================================
// Free-standing helper functions
// =========================================================================

/// Convert a `FieldType` into its `TokenStream` representation.
///
/// When `check_sizes` is true and the type is a `TypeName` whose name is not
/// yet in `known_sizes`, it is wrapped in `Box<…>`.
fn quote_field_type(
	state: &EmitterState,
	ft: &FieldType,
	check_sizes: bool,
) -> TokenStream {
	match ft {
		FieldType::Unit => quote! { () },
		FieldType::Bool => quote! { bool },
		FieldType::I8 => quote! { i8 },
		FieldType::I16 => quote! { i16 },
		FieldType::I32 => quote! { i32 },
		FieldType::I64 => quote! { i64 },
		FieldType::I128 => quote! { i128 },
		FieldType::U8 => quote! { u8 },
		FieldType::U16 => quote! { u16 },
		FieldType::U32 => quote! { u32 },
		FieldType::U64 => quote! { u64 },
		FieldType::U128 => quote! { u128 },
		FieldType::F32 => quote! { f32 },
		FieldType::F64 => quote! { f64 },
		FieldType::Char => quote! { char },
		FieldType::Str => quote! { String },
		FieldType::Bytes => {
			let ident = Ident::new("Bytes", Span::call_site());
			quote! { #ident }
		}
		FieldType::Option(inner) => {
			let inner_ty = quote_field_type(state, inner, check_sizes);
			quote! { Option<#inner_ty> }
		}
		FieldType::Seq(inner) => {
			let inner_ty = quote_field_type(state, inner, false);
			quote! { Vec<#inner_ty> }
		}
		FieldType::Map { key, value } => {
			let key_ty = quote_field_type(state, key, false);
			let val_ty = quote_field_type(state, value, false);
			let map_ident = Ident::new("Map", Span::call_site());
			quote! { #map_ident<#key_ty, #val_ty> }
		}
		FieldType::Tuple(fmts) => {
			let types: Vec<_> = fmts
				.iter()
				.map(|field_type| {
					quote_field_type(state, field_type, check_sizes)
				})
				.collect();
			quote! { (#(#types),*) }
		}
		FieldType::TupleArray { content, size } => {
			let inner = quote_field_type(state, content, check_sizes);
			let sz = *size;
			quote! { [#inner; #sz] }
		}
		FieldType::TypeName(type_name) => {
			let display_name = rename_type(state, type_name);
			let ident = Ident::new(&display_name, Span::call_site());
			if check_sizes
				&& !state.known_sizes.contains(type_name.as_str())
				&& !type_name.starts_with("Vec")
			{
				quote! { Box<#ident> }
			} else {
				quote! { #ident }
			}
		}
	}
}

/// Map a `FieldType` to its token representation for builder parameters.
///
/// This is used by the builder generator and intentionally mirrors
/// `quote_field_type` but operates independently of the `EmitterState`
/// sizing checks.
fn format_to_type_tokens(ft: &FieldType, title_case: bool) -> TokenStream {
	match ft {
		FieldType::TypeName(name) => {
			let display = if title_case {
				name.to_upper_camel_case()
			} else {
				name.clone()
			};
			let ident = Ident::new(&display, Span::call_site());
			quote! { #ident }
		}
		FieldType::Unit => quote! { () },
		FieldType::Bool => quote! { bool },
		FieldType::I8 => quote! { i8 },
		FieldType::I16 => quote! { i16 },
		FieldType::I32 => quote! { i32 },
		FieldType::I64 => quote! { i64 },
		FieldType::I128 => quote! { i128 },
		FieldType::U8 => quote! { u8 },
		FieldType::U16 => quote! { u16 },
		FieldType::U32 => quote! { u32 },
		FieldType::U64 => quote! { u64 },
		FieldType::U128 => quote! { u128 },
		FieldType::F32 => quote! { f32 },
		FieldType::F64 => quote! { f64 },
		FieldType::Char => quote! { char },
		FieldType::Str => quote! { String },
		FieldType::Bytes => {
			let ident = Ident::new("Bytes", Span::call_site());
			quote! { #ident }
		}
		FieldType::Option(inner) => {
			let inner_ty = format_to_type_tokens(inner, title_case);
			quote! { Option<#inner_ty> }
		}
		FieldType::Seq(inner) => {
			let inner_ty = format_to_type_tokens(inner, title_case);
			quote! { Vec<#inner_ty> }
		}
		FieldType::Map { key, value } => {
			let key_ty = format_to_type_tokens(key, title_case);
			let val_ty = format_to_type_tokens(value, title_case);
			let map_ident = Ident::new("Map", Span::call_site());
			quote! { #map_ident<#key_ty, #val_ty> }
		}
		FieldType::Tuple(fmts) => {
			let types: Vec<_> = fmts
				.iter()
				.map(|field_type| format_to_type_tokens(field_type, title_case))
				.collect();
			quote! { (#(#types),*) }
		}
		FieldType::TupleArray { content, size } => {
			let inner = format_to_type_tokens(content, title_case);
			let sz = *size;
			quote! { [#inner; #sz] }
		}
	}
}

/// Return a token stream producing the natural default value for a given
/// field type (used for optional fields in generated constructors).
fn default_value_tokens(ft: &FieldType) -> TokenStream {
	match ft {
		FieldType::Option(_) => quote! { None },
		FieldType::Seq(_) => quote! { Vec::new() },
		FieldType::Str => quote! { String::new() },
		FieldType::Bool => quote! { false },
		FieldType::I8
		| FieldType::I16
		| FieldType::I32
		| FieldType::I64
		| FieldType::I128 => {
			quote! { 0 }
		}
		FieldType::U8
		| FieldType::U16
		| FieldType::U32
		| FieldType::U64
		| FieldType::U128 => {
			quote! { 0 }
		}
		FieldType::F32 | FieldType::F64 => quote! { 0.0 },
		FieldType::Map { .. } => {
			let map_ident = Ident::new("Map", Span::call_site());
			quote! { #map_ident::new() }
		}
		_ => quote! { Default::default() },
	}
}

/// Apply the title-case rename map to a type name string.
fn rename_type(state: &EmitterState, name: &str) -> String {
	if state.type_renames.is_empty() {
		return name.to_string();
	}
	// Fast path: direct lookup
	if let Some(renamed) = state.type_renames.get(name) {
		return renamed.clone();
	}
	// Slow path: replace all known names inside a compound expression.
	let mut result = name.to_string();
	let mut sorted: Vec<_> = state.type_renames.iter().collect();
	sorted.sort_by(|entry_a, entry_b| entry_b.0.len().cmp(&entry_a.0.len()));
	for (old, new) in sorted {
		result = result.replace(old.as_str(), new.as_str());
	}
	result
}

/// Compute the final struct name for a given registry entry, applying
/// namespace prefixing and title-case conversion as appropriate.
fn resolve_struct_name(
	state: &EmitterState,
	namespace: &Option<String>,
	name: &str,
) -> String {
	let raw = match namespace {
		Some(ns) => format!("{}_{}", ns, name),
		None => name.to_string(),
	};
	rename_type(state, &raw)
}

/// Map a provider source string to the `TerraProvider` constant name.
fn provider_source_to_const(source: &str) -> String {
	let provider_name = source.split('/').last().unwrap_or("unknown");
	provider_name.to_uppercase()
}

/// Produce `#[serde(skip_serializing_if = "…")]` annotations.
fn field_serde_annotation(ft: &FieldType) -> TokenStream {
	match ft {
		FieldType::Str => {
			quote! { #[serde(skip_serializing_if = "String::is_empty")] }
		}
		FieldType::Option(_) => {
			quote! { #[serde(skip_serializing_if = "Option::is_none")] }
		}
		FieldType::Seq(_) => {
			quote! { #[serde(skip_serializing_if = "Vec::is_empty")] }
		}
		_ => quote! {},
	}
}

/// Create a field identifier, handling `r#` raw identifiers for reserved words.
fn make_field_ident(name: &str) -> Ident {
	if name.starts_with("r#") {
		// Already raw — syn/proc-macro2 need the name without the r# prefix
		// but created via `format_ident!("r#{}", …)`.
		let raw_name = &name[2..];
		format_ident!("r#{}", raw_name)
	} else {
		Ident::new(name, Span::call_site())
	}
}

/// Build a list of derive macro idents from string names.
fn derive_idents(names: &[String]) -> Vec<Ident> {
	names
		.iter()
		.map(|name| Ident::new(name, Span::call_site()))
		.collect()
}

/// Emit a doc-comment token stream from the comments map.
fn emit_doc_from_comments(
	comments: &std::collections::BTreeMap<Vec<String>, String>,
	path: &[String],
) -> TokenStream {
	if let Some(doc) = comments.get(path) {
		emit_doc_string(doc)
	} else {
		quote! {}
	}
}

/// Emit a `#[doc = "…"]` attribute for each line of a doc string.
fn emit_doc_string(doc: &str) -> TokenStream {
	let trimmed = doc.trim();
	if trimmed.is_empty() {
		return quote! {};
	}
	let lines: Vec<_> = trimmed.lines().collect();
	let attrs: Vec<TokenStream> = lines
		.iter()
		.map(|line| {
			let line_str = format!(" {}", line);
			quote! { #[doc = #line_str] }
		})
		.collect();
	quote! { #(#attrs)* }
}

/// Convert a `TokenStream` into formatted Rust source code via `prettyplease`.
fn tokens_to_source(tokens: TokenStream) -> Result<String> {
	let source_text = tokens.to_string();
	let file = syn::parse_file(&source_text).map_err(|err| {
		bevyhow!("syn parse error: {}\n--- source ---\n{}", err, source_text)
	})?;
	Ok(prettyplease::unparse(&file))
}
