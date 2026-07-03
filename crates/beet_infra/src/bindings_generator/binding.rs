use super::config::CodeGeneratorConfig;
use super::config::DocComments;
use super::emit::CodeGenerator;
use super::ir::Container;
use super::ir::Field;
use super::ir::FieldType;
use super::ir::Registry;
use super::ir::Variant;
use super::ir::VariantFormat;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

pub const RESERVED_WORDS: [&str; 32] = [
	"as",
	"break",
	"pub const",
	"continue",
	"else",
	"enum",
	"false",
	"fn",
	"for",
	"if",
	"impl",
	"in",
	"let",
	"loop",
	"match",
	"mod",
	"mut",
	"ref",
	"return",
	"self",
	"Self",
	"static",
	"super",
	"trait",
	"true",
	"type",
	"unsafe",
	"use",
	"where",
	"while",
	"const",
	"box",
];

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum Void {}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TerraformSchemaExport {
	provider_schemas: BTreeMap<String, Schema>,
	format_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Schema {
	provider: Provider,
	data_source_schemas: Option<BTreeMap<String, SchemaItem>>,
	resource_schemas: Option<BTreeMap<String, SchemaItem>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Provider {
	version: i64,
	block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct SchemaItem {
	version: i64,
	block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Block {
	attributes: Option<BTreeMap<String, Attribute>>,
	block_types: Option<BTreeMap<String, NestedBlock>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
enum StringKind {
	Plain,
	Markdown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Attribute {
	r#type: Option<AttributeType>,
	description: Option<String>,
	required: Option<bool>,
	optional: Option<bool>,
	computed: Option<bool>,
	sensitive: Option<bool>,
	description_kind: Option<StringKind>,
	deprecated: Option<bool>,
	/// Present when the attribute uses an inline structural type instead of
	/// `type` (the plugin-framework style modern providers emit, eg cloudflare
	/// v5). Exported as its own struct container, nested recursively.
	nested_type: Option<NestedType>,
}

/// A plugin-framework structural attribute type: a full attribute map plus how
/// it nests (`single`, `list`, `set` or `map`).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct NestedType {
	attributes: Option<BTreeMap<String, Attribute>>,
	nesting_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct NestedBlock {
	block: Block,
	nesting_mode: Option<String>,
	min_items: Option<u8>,
	max_items: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AttributeType(Value);

pub fn generate_serde(
	config: &str,
	out: &mut dyn Write,
	registry: &Registry,
) -> Result {
	let config = CodeGeneratorConfig::new()
		.with_module_name(config)
		.with_generate_roots(true);

	CodeGenerator::new(&config).output(out, registry)
}

pub fn export_schema_to_registry(
	schema: &TerraformSchemaExport,
) -> Result<Registry> {
	let mut registry = Registry::new();
	let mut roots = BTreeMap::new();
	roots.insert("provider", Vec::<&str>::new());
	roots.insert("resource", Vec::<&str>::new());
	roots.insert("data", Vec::<&str>::new());

	for (pn, pv) in &schema.provider_schemas {
		let pn = pn.split('/').last().unwrap_or(pn);
		let ps = &pv.provider;
		export_block(None, pn, ps.block.clone(), &mut registry)?;
		if let Some(provider) = roots.get_mut("provider") {
			provider.push(pn);
		}

		if let Some(rss) = &pv.resource_schemas {
			for (name, item) in rss {
				// add terraform meta-tags to block
				let mut block = item.block.clone();
				inject_meta_arguments(&mut block);

				export_block(
					Some("resource".to_owned()),
					name,
					block,
					&mut registry,
				)?;
				if let Some(resources) = roots.get_mut("resource") {
					resources.push(name);
				}
			}
		}

		if let Some(dss) = &pv.data_source_schemas {
			for (name, item) in dss {
				let block = item.block.clone();
				export_block(
					Some("data_source".to_owned()),
					name,
					block,
					&mut registry,
				)?;
				if let Some(resources) = roots.get_mut("data") {
					resources.push(name);
				}
			}
		}

		export_roots(&roots, &mut registry);
		generate_config(&roots, &mut registry);
	}
	Ok(registry)
}

/// Export only the resources that pass through `filter`, skipping root enums
/// and the top-level `config` struct.
/// Returns the registry, metadata about every generated resource, and
/// collected doc comments extracted from schema `description` fields.
pub fn export_filtered_resources(
	schema: &TerraformSchemaExport,
	filter: &terra::ResourceFilter,
	config: &CodeGeneratorConfig,
) -> Result<(Registry, Vec<terra::ResourceMeta>, DocComments)> {
	let mut registry = Registry::new();
	let mut meta = Vec::new();
	let mut comments = DocComments::new();

	for (provider_source, provider_schema) in &schema.provider_schemas {
		if !filter.has_provider(provider_source) {
			continue;
		}

		if let Some(resource_schemas) = &provider_schema.resource_schemas {
			for (resource_name, schema_item) in resource_schemas {
				if !filter.allows(provider_source, resource_name) {
					continue;
				}

				let mut block = schema_item.block.clone();
				inject_meta_arguments(&mut block);

				let container_name = format!("{}_details", resource_name);
				collect_descriptions(
					config.module_name_str(),
					&container_name,
					&block,
					&mut comments,
				);

				export_block(
					Some("resource".to_owned()),
					resource_name,
					block,
					&mut registry,
				)?;

				use heck::ToUpperCamelCase;

				let struct_name = if config.use_title_case {
					container_name.to_upper_camel_case()
				} else {
					container_name
				};

				meta.push(terra::ResourceMeta {
					resource_type: resource_name.clone(),
					provider_source: provider_source.clone(),
					struct_name,
				});
			}
		}
	}

	Ok((registry, meta, comments))
}

fn generate_config(roots: &BTreeMap<&str, Vec<&str>>, reg: &mut Registry) {
	let mut target_attrs = Vec::new();

	for root_name in roots.keys() {
		target_attrs.push(Field::new(
			root_name.to_string(),
			FieldType::Option(Box::new(FieldType::Seq(Box::new(
				FieldType::TypeName(format!("{}_root", root_name)),
			)))),
		));
	}
	reg.insert(
		(None, "config".to_string()),
		Container::Struct(target_attrs),
	);
}

fn export_roots(roots: &BTreeMap<&str, Vec<&str>>, reg: &mut Registry) {
	for (root_name, root_members) in roots {
		let mut enumz = BTreeMap::new();
		for (pos, member) in root_members.iter().enumerate() {
			let details_type =
				FieldType::TypeName(format!("{}_details", member));

			let variant_type = if root_name.to_string().eq("provider") {
				// Vec<{member}_details>
				FieldType::Seq(Box::new(details_type))
			} else {
				// Vec<Map<String, Vec<{member}_details>>>
				FieldType::Seq(Box::new(FieldType::Map {
					key: Box::new(FieldType::Str),
					value: Box::new(FieldType::Seq(Box::new(details_type))),
				}))
			};

			enumz.insert(
				pos as u32,
				Variant::new(
					member.to_string(),
					VariantFormat::NewType(Box::new(variant_type)),
				),
			);
		}
		reg.insert(
			(None, format!("{}_root", root_name.to_owned())),
			Container::Enum(enumz),
		);
	}
}

/// Walk a block and collect `description` values into the doc comments map.
///
/// Keys are `[module_name, container_name, field_name]` to match the format
/// expected by `RustEmitter::output_comment`.
fn collect_descriptions(
	module_name: &str,
	container_name: &str,
	block: &Block,
	comments: &mut DocComments,
) {
	if let Some(attrs) = &block.attributes {
		collect_attribute_descriptions(
			module_name,
			container_name,
			attrs,
			comments,
		);
	}

	if let Some(block_types) = &block.block_types {
		for (bt_name, nested) in block_types {
			let nested_container =
				format!("{}_block_type_{}", container_name, bt_name);
			collect_descriptions(
				module_name,
				&nested_container,
				&nested.block,
				comments,
			);
		}
	}
}

/// Collect an attribute map's descriptions, recursing into `nested_type`
/// attribute maps (their containers are named `{container}_{attr}`, matching
/// [`export_nested_type`]).
fn collect_attribute_descriptions(
	module_name: &str,
	container_name: &str,
	attrs: &BTreeMap<String, Attribute>,
	comments: &mut DocComments,
) {
	for (attr_name, attr) in attrs {
		let mut doc = attr.description.clone().unwrap_or_default();
		let meta = super::ir::FieldMetadata {
			required: attr.required.unwrap_or(false),
			optional: attr.optional.unwrap_or(false),
			computed: attr.computed.unwrap_or(false),
			sensitive: attr.sensitive.unwrap_or(false),
		};
		if let Some(flags) = meta.flags_doc() {
			if !doc.is_empty() {
				doc.push('\n');
			}
			doc.push_str(&format!("## Attribute\n{}", flags));
		}
		if !doc.is_empty() {
			let key = vec![
				module_name.to_string(),
				container_name.to_string(),
				attr_name.clone(),
			];
			comments.insert(key, doc);
		}
		if let Some(nested_attrs) = attr
			.nested_type
			.as_ref()
			.and_then(|nested| nested.attributes.as_ref())
		{
			collect_attribute_descriptions(
				module_name,
				&format!("{container_name}_{attr_name}"),
				nested_attrs,
				comments,
			);
		}
	}
}

/// Sanitize an attribute name for use as a Rust field identifier: `self` /
/// `Self` (which cannot be raw identifiers) are renamed, other reserved words
/// get the `r#` prefix.
fn sanitize_field_name(name: &str) -> String {
	if name == "self" {
		"self_ref".to_string()
	} else if name == "Self" {
		"Self_ref".to_string()
	} else {
		RESERVED_WORDS
			.iter()
			.find(|word| name == &word.to_string())
			.map(|word| format!("r#{}", word))
			.unwrap_or_else(|| name.to_string())
	}
}

/// Export an attribute map as a struct container, registering a nested
/// container for every structural attribute (`nested_type` or a cty
/// `object`) under `{parent_fqn}_{attr}`.
fn export_attributes(
	parent_fqn: &str,
	attrs: &BTreeMap<String, Attribute>,
	reg: &mut Registry,
) -> Result<Option<Container>> {
	let mut target_attrs = Vec::new();
	for (an, at) in attrs {
		let field_type = match (&at.nested_type, &at.r#type) {
			// plugin-framework structural attribute: a full nested attribute map
			(Some(nested), _) => {
				export_nested_type(parent_fqn, an, nested, reg)?
			}
			(None, Some(AttributeType(value))) => {
				cty_field_type(parent_fqn, an, value, reg)?
			}
			(None, None) => {
				bevybail!("attribute `{parent_fqn}.{an}` has no type");
			}
		};
		let attr_fmt = match (at.optional, at.computed) {
			(Some(opt), _) if opt => {
				FieldType::Option(Box::new(field_type.clone()))
			}
			(_, Some(cmp)) if cmp => {
				FieldType::Option(Box::new(field_type.clone()))
			}
			_ => field_type.clone(),
		};

		let metadata = super::ir::FieldMetadata {
			required: at.required.unwrap_or(false),
			optional: at.optional.unwrap_or(false),
			computed: at.computed.unwrap_or(false),
			sensitive: at.sensitive.unwrap_or(false),
		};
		target_attrs.push(Field::with_metadata(
			sanitize_field_name(an),
			attr_fmt,
			metadata,
		));
	}
	if !target_attrs.is_empty() {
		Ok(Some(Container::Struct(target_attrs)))
	} else {
		Ok(None)
	}
}

/// Register the container for a plugin-framework `nested_type` attribute and
/// return the field type referencing it, shaped by `nesting_mode`.
fn export_nested_type(
	parent_fqn: &str,
	attr_name: &str,
	nested: &NestedType,
	reg: &mut Registry,
) -> Result<FieldType> {
	let fqn = format!("{parent_fqn}_{attr_name}");
	let container = nested
		.attributes
		.as_ref()
		.map(|attrs| export_attributes(&fqn, attrs, reg))
		.transpose()?
		.flatten()
		.unwrap_or(Container::Struct(Vec::new()));
	insert_nested_container(&fqn, container, reg)?;
	let type_name = FieldType::TypeName(fqn);
	match nested.nesting_mode.as_deref() {
		Some("list") | Some("set") => Ok(FieldType::Seq(Box::new(type_name))),
		Some("map") => Ok(FieldType::Map {
			key: Box::new(FieldType::Str),
			value: Box::new(type_name),
		}),
		Some("single") | None => Ok(type_name),
		Some(other) => {
			bevybail!(
				"attribute `{parent_fqn}.{attr_name}` has unknown nesting_mode `{other}`"
			)
		}
	}
}

/// Convert a cty type value (the `type` field of a legacy-SDK attribute) into
/// a [`FieldType`], registering a struct container for every `object` under
/// `{parent_fqn}_{attr_name}`.
fn cty_field_type(
	parent_fqn: &str,
	attr_name: &str,
	value: &Value,
	reg: &mut Registry,
) -> Result<FieldType> {
	match value {
		Value::String(type_str) => match type_str.as_str() {
			"string" => Ok(FieldType::Str),
			"bool" => Ok(FieldType::Bool),
			"number" => Ok(FieldType::I64),
			// "dynamic" can hold any value; approximate as a string
			"dynamic" => Ok(FieldType::Str),
			// bare collection names (the injected meta-arguments): element
			// type unknown, approximate as strings
			"set" | "list" => Ok(FieldType::Seq(Box::new(FieldType::Str))),
			"map" => Ok(FieldType::Map {
				key: Box::new(FieldType::Str),
				value: Box::new(FieldType::Str),
			}),
			other => bevybail!("Unknown type {other}"),
		},
		Value::Array(type_arr) => {
			let kind = type_arr
				.first()
				.and_then(Value::as_str)
				.ok_or_else(|| bevyhow!("empty cty type array"))?;
			match kind {
				"set" | "list" => match type_arr.get(1) {
					Some(elem) => {
						cty_field_type(parent_fqn, attr_name, elem, reg)?
							.xmap(Box::new)
							.xmap(FieldType::Seq)
							.xok()
					}
					None => Ok(FieldType::Seq(Box::new(FieldType::Str))),
				},
				"map" => match type_arr.get(1) {
					Some(elem) => Ok(FieldType::Map {
						key: Box::new(FieldType::Str),
						value: cty_field_type(
							parent_fqn, attr_name, elem, reg,
						)?
						.xmap(Box::new),
					}),
					None => Ok(FieldType::Map {
						key: Box::new(FieldType::Str),
						value: Box::new(FieldType::Str),
					}),
				},
				"object" => {
					let Some(Value::Object(attrs)) = type_arr.get(1) else {
						bevybail!(
							"cty object at `{parent_fqn}.{attr_name}` has no attribute map"
						);
					};
					// the optional third element lists the optional attribute names
					let optional_names = type_arr
						.get(2)
						.and_then(Value::as_array)
						.map(|names| {
							names
								.iter()
								.filter_map(Value::as_str)
								.collect::<std::collections::HashSet<_>>()
						})
						.unwrap_or_default();
					let fqn = format!("{parent_fqn}_{attr_name}");
					let fields = attrs
						.iter()
						.map(|(field_name, field_value)| {
							let field_type = cty_field_type(
								&fqn,
								field_name,
								field_value,
								reg,
							)?;
							let field_type = if optional_names
								.contains(field_name.as_str())
							{
								FieldType::Option(Box::new(field_type))
							} else {
								field_type
							};
							Field::new(
								sanitize_field_name(field_name),
								field_type,
							)
							.xok()
						})
						.collect::<Result<Vec<_>>>()?;
					insert_nested_container(
						&fqn,
						Container::Struct(fields),
						reg,
					)?;
					Ok(FieldType::TypeName(fqn))
				}
				// a positional tuple of cty types
				"tuple" => {
					let elems = type_arr
						.get(1)
						.and_then(Value::as_array)
						.ok_or_else(|| {
							bevyhow!(
								"cty tuple at `{parent_fqn}.{attr_name}` has no element list"
							)
						})?
						.iter()
						.map(|elem| {
							cty_field_type(parent_fqn, attr_name, elem, reg)
						})
						.collect::<Result<Vec<_>>>()?;
					Ok(FieldType::Tuple(elems))
				}
				other => bevybail!("Unknown cty type kind `{other}`"),
			}
		}
		unknown => bevybail!("Type {unknown:?} not supported"),
	}
}

/// Insert a nested container, failing loudly on a name collision with a
/// *different* definition (two attribute paths flattening to the same fqn)
/// rather than silently overwriting one of them.
fn insert_nested_container(
	fqn: &str,
	container: Container,
	reg: &mut Registry,
) -> Result {
	let key = (None, fqn.to_string());
	if let Some(existing) = reg.get(&key)
		&& existing != &container
	{
		bevybail!("nested container name collision at `{fqn}`");
	}
	reg.insert(key, container);
	Ok(())
}

fn inject_meta_arguments(blk: &mut Block) {
	let depends_on_attr = Attribute {
		r#type: Some(AttributeType(serde_json::json!(["set"]))),
		optional: Some(true),
		..Default::default()
	};
	let count_attr = Attribute {
		r#type: Some(AttributeType(serde_json::json!("number"))),
		optional: Some(true),
		..Default::default()
	};

	let for_each_attr = Attribute {
		r#type: Some(AttributeType(serde_json::json!(["set"]))),
		optional: Some(true),
		..Default::default()
	};

	let provider_attr = Attribute {
		r#type: Some(AttributeType(serde_json::json!("string"))),
		optional: Some(true),
		..Default::default()
	};

	if let Some(attrs) = blk.attributes.as_mut() {
		attrs.insert("depends_on".to_owned(), depends_on_attr);
		attrs.insert("count".to_owned(), count_attr);
		attrs.insert("for_each".to_owned(), for_each_attr);
		attrs.insert("provider".to_owned(), provider_attr);
	}
}

fn export_block(
	namespace: Option<String>,
	name: &str,
	blk: Block,
	reg: &mut Registry,
) -> Result {
	let mut cf1 =
		export_attributes(name, blk.attributes.as_ref().unwrap(), reg)?;
	if let Some(bt) = &blk.block_types {
		for (block_type_name, nested_block) in bt {
			export_block_type(
				namespace.as_ref(),
				name,
				block_type_name,
				nested_block,
				reg,
				cf1.as_mut().unwrap(),
			)?;
		}
	}

	reg.insert((None, format!("{}_details", name)), cf1.unwrap());

	Ok(())
}

fn export_block_type(
	namespace: Option<&String>,
	parent_name: &str,
	name: &str,
	blk: &NestedBlock,
	reg: &mut Registry,
	cf: &mut Container,
) -> Result {
	let mut inner_block_types = Vec::new();
	if let Some(attrs) = &blk.block.attributes {
		let block_type_ns = namespace.map_or_else(
			|| format!("{}_block_type", parent_name),
			|val| format!("{}_{}_block_type", parent_name, val),
		);
		let block_type_fqn = namespace.map_or_else(
			|| format!("{}_block_type_{}", parent_name, name.to_owned()),
			|val| {
				format!(
					"{}_{}_block_type_{}",
					parent_name,
					val,
					name.to_owned()
				)
			},
		);
		let mut nested_cf = export_attributes(&block_type_fqn, attrs, reg)?;

		// export inner block types
		if let Some(bt) = &blk.block.block_types {
			for (block_type_name, nested_block) in bt {
				export_block_type(
					namespace,
					name,
					block_type_name,
					nested_block,
					reg,
					nested_cf.as_mut().unwrap(),
				)?;
			}
		}
		reg.insert((Some(block_type_ns), name.to_owned()), nested_cf.unwrap());
		inner_block_types.push((name, block_type_fqn));
	}

	if let Container::Struct(attrs) = cf {
		for (_, (name, fqn)) in inner_block_types.iter().enumerate() {
			attrs.push(Field::new(
				name.to_string(),
				FieldType::Option(Box::new(FieldType::Seq(Box::new(
					FieldType::TypeName(fqn.to_string()),
				)))),
			));
		}
	};

	Ok(())
}

/// Read a Terraform schema export from a JSON file on disk.
pub fn read_tf_schema_from_file<P: AsRef<Path>>(
	path: P,
) -> Result<TerraformSchemaExport> {
	let bytes = fs_ext::read(path)?;
	let schema: TerraformSchemaExport = serde_json::from_slice(&bytes)?;
	Ok(schema)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::bindings_generator::test_utils::config;
	use crate::bindings_generator::test_utils::datasource_root;
	use crate::bindings_generator::test_utils::provider_root;
	use crate::bindings_generator::test_utils::resource_root;

	fn fixtures_dir() -> std::path::PathBuf {
		fs_ext::workspace_root().join("crates/beet_infra/tests/fixtures")
	}

	#[beet_core::test]
	fn deserialize_example_tf_schema() {
		let tf_schema = read_tf_schema_from_file(
			fixtures_dir().join("test-provider-schema.json"),
		);

		tf_schema.is_ok().xpect_true();
		let test_schema = tf_schema
			.as_ref()
			.unwrap()
			.provider_schemas
			.get("test_provider");

		tf_schema
			.as_ref()
			.unwrap()
			.provider_schemas
			.len()
			.xpect_eq(1);
		test_schema.is_some().xpect_true();
		test_schema
			.unwrap()
			.data_source_schemas
			.as_ref()
			.unwrap()
			.len()
			.xpect_eq(2);
		test_schema
			.map(|schema| schema.resource_schemas.is_none())
			.xpect_eq(Some(false));
	}

	#[beet_core::test]
	fn generate_registry_from_schema() {
		let tf_schema = read_tf_schema_from_file(
			fixtures_dir().join("test-provider-schema.json"),
		);
		let registry = export_schema_to_registry(&tf_schema.as_ref().unwrap());

		registry.is_ok().xpect_true();
		registry.unwrap().len().xpect_eq(10);
	}

	#[beet_core::test(timeout_ms = 120000)]
	async fn generate_serde_model_from_registry() {
		let tf_schema = read_tf_schema_from_file(
			fixtures_dir().join("test-provider-schema.json"),
		);
		let registry = export_schema_to_registry(&tf_schema.as_ref().unwrap());
		let dir = TempDir::new().unwrap();

		fs_ext::write(
			dir.as_ref().join("Cargo.toml"),
			r#"[package]
    name = "testing"
    version = "0.1.0"
    edition = "2018"

    [dependencies]
    serde = { version = "1.0", features = ["derive"] }
    serde_bytes = "0.11"
    smol_str = { version = "0.2", features = ["serde"] }

    [workspace]
    "#,
		)
		.unwrap();
		fs_ext::create_dir_all(dir.as_ref().join("src")).unwrap();
		let source_path = dir.as_ref().join("src/lib.rs");
		let mut source_buf = Vec::new();
		generate_serde("test", &mut source_buf, &registry.unwrap()).unwrap();
		// Inject stubs so the generated preamble resolves in the temp crate:
		// - `use beet_core::prelude::*` → provides `SmolStr`
		// - `use crate::prelude::*`     → empty module, satisfies the import
		let generated = String::from_utf8(source_buf).unwrap();
		let preamble_stubs = concat!(
			"pub mod beet_core {\n",
			"    pub mod prelude {\n",
			"        pub use smol_str::SmolStr;\n",
			"    }\n",
			"}\n",
			"pub mod prelude {}\n",
		);
		// Insert after the first line (the `#![allow(…)]` inner attribute).
		let source_with_stub = if let Some(nl) = generated.find('\n') {
			let (first_line, rest) = generated.split_at(nl + 1);
			format!("{}{}{}", first_line, preamble_stubs, rest)
		} else {
			format!("{}{}", preamble_stubs, generated)
		};
		fs_ext::write(&source_path, source_with_stub).unwrap();
		// Use a stable `target` dir to avoid downloading and recompiling crates every time.
		let target_dir = fs_ext::workspace_root().join("target");
		let output = async_process::Command::new("cargo")
			.current_dir(dir.as_ref())
			.arg("build")
			.arg("--target-dir")
			.arg(target_dir)
			.output()
			.await
			.unwrap();
		output.status.success().xpect_true();
	}

	#[beet_core::test]
	fn unmarshall_provider() {
		let res: config = serde_json::from_str(include_str!(
			"../../tests/fixtures/provider_test.json"
		))
		.unwrap();
		res.provider
			.as_ref()
			.map(|providers| providers.is_empty())
			.xpect_eq(Some(false));
		res.provider
			.as_ref()
			.map(|providers| providers.get(0).is_none())
			.xpect_eq(Some(false));
		let prv = res
			.provider
			.as_ref()
			.and_then(|providers| providers.get(0))
			.and_then(|entry| match entry {
				provider_root::test_provider(provider) => provider.get(0),
			});
		prv.is_none().xpect_eq(false);
		prv.map(|provider| provider.api_token.to_owned())
			.xpect_eq(Some("ABC12345".to_owned()));
	}

	#[beet_core::test]
	fn unmarshall_resource() {
		let res: config = serde_json::from_str(include_str!(
			"../../tests/fixtures/resource_test.json"
		))
		.unwrap();
		res.resource
			.as_ref()
			.map(|resources| resources.is_empty())
			.xpect_eq(Some(false));
		res.resource
			.as_ref()
			.map(|resources| resources.get(0).is_none())
			.xpect_eq(Some(false));
		let res_a = res
			.resource
			.as_ref()
			.and_then(|resources| resources.get(0))
			.and_then(|entry| match entry {
				resource_root::test_resource_a(r1) => r1.get(0),
				_ => None,
			})
			.and_then(|map| map.get("test"))
			.and_then(|entries| entries.first());
		res_a.is_none().xpect_eq(false);
		res_a
			.map(|resource| resource.name.to_owned())
			.xpect_eq(Some("test_resource_a".to_owned()));
	}

	#[beet_core::test]
	fn unmarshall_datasource() {
		let res: config = serde_json::from_str(include_str!(
			"../../tests/fixtures/datasource_test.json"
		))
		.unwrap();
		res.data
			.as_ref()
			.map(|data| data.is_empty())
			.xpect_eq(Some(false));
		res.data
			.as_ref()
			.map(|data| data.get(0).is_none())
			.xpect_eq(Some(false));
		let res_a = res
			.data
			.as_ref()
			.and_then(|data| data.get(0))
			.and_then(|entry| match entry {
				datasource_root::test_data_source_b(ds1) => ds1.get(0),
				_ => None,
			})
			.and_then(|map| map.get("test"))
			.and_then(|entries| entries.first());
		res_a.is_none().xpect_eq(false);
		res_a
			.map(|datasource| datasource.name.to_owned())
			.xpect_eq(Some("test_datasource_b".to_owned()));
	}

	#[beet_core::test]
	fn unmarshall_block_type() {
		let res: config = serde_json::from_str(include_str!(
			"../../tests/fixtures/block_type_test.json"
		))
		.unwrap();
		res.data
			.as_ref()
			.map(|data| data.is_empty())
			.xpect_eq(Some(false));
		res.data
			.as_ref()
			.map(|data| data.get(0).is_none())
			.xpect_eq(Some(false));
		let res_a = res
			.data
			.as_ref()
			.and_then(|data| data.get(0))
			.and_then(|entry| match entry {
				datasource_root::test_data_source_a(ds1) => ds1.get(0),
				_ => None,
			})
			.and_then(|map| map.get("test"))
			.and_then(|entries| entries.first());
		res_a.is_none().xpect_eq(false);
		res_a
			.map(|datasource| datasource.name.to_owned())
			.xpect_eq(Some("test_datasource_a".to_owned()));
		res_a
			.map(|datasource| datasource.datasource_a_type.is_none())
			.xpect_eq(Some(false));
		res_a
			.and_then(|datasource| {
				datasource
					.datasource_a_type
					.as_ref()
					.map(|types| types.is_empty())
			})
			.xpect_eq(Some(false));
		res_a
			.and_then(|datasource| {
				datasource
					.datasource_a_type
					.as_ref()
					.unwrap()
					.first()
					.unwrap()
					.filter_type
					.to_owned()
			})
			.xpect_eq(Some("REGEX".to_owned()));
	}

	/// A mini schema exercising the structural shapes: a `nested_type` list
	/// (plugin-framework style) holding a `single` nested object, a cty
	/// `object` with optional attribute names, and a cty list-of-object.
	fn nested_schema() -> TerraformSchemaExport {
		serde_json::from_value(serde_json::json!({
			"format_version": "1.0",
			"provider_schemas": {
				"registry.opentofu.org/cloudflare/cloudflare": {
					"provider": { "version": 0, "block": { "attributes": {} } },
					"resource_schemas": {
						"test_nested": {
							"version": 0,
							"block": {
								"attributes": {
									"zone_id": { "type": "string", "required": true },
									"rules": {
										"optional": true,
										"nested_type": {
											"nesting_mode": "list",
											"attributes": {
												"expression": { "type": "string", "required": true },
												"action_parameters": {
													"optional": true,
													"nested_type": {
														"nesting_mode": "single",
														"attributes": {
															"cache": { "type": "bool", "optional": true }
														}
													}
												}
											}
										}
									},
									"origin": {
										"optional": true,
										"type": ["object", { "name": "string", "port": "number" }, ["port"]]
									},
									"items": {
										"optional": true,
										"type": ["list", ["object", { "value": "string" }]]
									}
								}
							}
						}
					}
				}
			}
		}))
		.unwrap()
	}

	fn nested_filter() -> terra::ResourceFilter {
		terra::ResourceFilter::default()
			.with_resources("registry.opentofu.org/cloudflare/cloudflare", &[
				"test_nested".to_string(),
			])
	}

	/// Structural attributes register their own containers and the fields
	/// reference them, shaped by nesting mode.
	#[beet_core::test]
	fn exports_nested_types() {
		let (registry, _meta, _comments) = export_filtered_resources(
			&nested_schema(),
			&nested_filter(),
			&CodeGeneratorConfig::new(),
		)
		.unwrap();

		for name in [
			"test_nested_details",
			"test_nested_rules",
			"test_nested_rules_action_parameters",
			"test_nested_origin",
			"test_nested_items",
		] {
			registry
				.contains_key(&(None, name.to_string()))
				.xpect_true();
		}

		let fields = registry[&(None, "test_nested_details".to_string())]
			.fields()
			.unwrap();
		let field = |name: &str| {
			fields.iter().find(|field| field.name == name).unwrap()
		};
		field("rules").value.xpect_eq(FieldType::Option(Box::new(
			FieldType::Seq(Box::new(FieldType::TypeName(
				"test_nested_rules".to_string(),
			))),
		)));
		field("origin").value.xpect_eq(FieldType::Option(Box::new(
			FieldType::TypeName("test_nested_origin".to_string()),
		)));
		field("items").value.xpect_eq(FieldType::Option(Box::new(
			FieldType::Seq(Box::new(FieldType::TypeName(
				"test_nested_items".to_string(),
			))),
		)));

		// the single-nested object nests recursively
		registry[&(None, "test_nested_rules".to_string())]
			.fields()
			.unwrap()
			.iter()
			.find(|field| field.name == "action_parameters")
			.unwrap()
			.value
			.xpect_eq(FieldType::Option(Box::new(FieldType::TypeName(
				"test_nested_rules_action_parameters".to_string(),
			))));

		// the cty object's third element marks `port` optional
		let origin_fields = registry[&(None, "test_nested_origin".to_string())]
			.fields()
			.unwrap();
		origin_fields
			.iter()
			.find(|field| field.name == "port")
			.unwrap()
			.value
			.xpect_eq(FieldType::Option(Box::new(FieldType::I64)));
		origin_fields
			.iter()
			.find(|field| field.name == "name")
			.unwrap()
			.value
			.xpect_eq(FieldType::Str);
	}

	/// The generated Rust references the typed structs, unboxed.
	#[beet_core::test]
	fn generates_nested_structs() {
		let output = crate::bindings_generator::BindingGenerator::new()
			.with_title_case(true)
			.with_trait_impls(true)
			.with_filter(nested_filter())
			.generate_to_string(&nested_schema())
			.unwrap();
		output
			.xpect_contains("pub struct TestNestedDetails")
			.xpect_contains("pub struct TestNestedRules")
			.xpect_contains("pub struct TestNestedRulesActionParameters")
			.xpect_contains("pub rules: Option<Vec<TestNestedRules>>")
			.xpect_contains(
				"pub action_parameters: Option<TestNestedRulesActionParameters>",
			)
			.xpect_contains("pub origin: Option<TestNestedOrigin>")
			.xpect_contains("pub items: Option<Vec<TestNestedItems>>")
			.xpect_contains("pub port: Option<i64>")
			.xpect_contains("impl terra::Resource for TestNestedDetails");
	}

	#[beet_core::test]
	fn handles_reserved_words_in_attributes() {
		// Test that 'self' and 'Self' are renamed to avoid raw identifier issues
		let mut attrs = BTreeMap::new();
		attrs.insert("self".to_string(), Attribute {
			r#type: Some(AttributeType(serde_json::Value::String(
				"bool".to_string(),
			))),
			description: None,
			required: Some(false),
			optional: Some(true),
			computed: None,
			sensitive: None,
			description_kind: None,
			deprecated: None,
			nested_type: None,
		});
		attrs.insert("Self".to_string(), Attribute {
			r#type: Some(AttributeType(serde_json::Value::String(
				"string".to_string(),
			))),
			description: None,
			required: Some(true),
			optional: None,
			computed: None,
			sensitive: None,
			description_kind: None,
			deprecated: None,
			nested_type: None,
		});
		attrs.insert("type".to_string(), Attribute {
			r#type: Some(AttributeType(serde_json::Value::String(
				"string".to_string(),
			))),
			description: None,
			required: Some(true),
			optional: None,
			computed: None,
			sensitive: None,
			description_kind: None,
			deprecated: None,
			nested_type: None,
		});

		let mut reg = Registry::new();
		let result = export_attributes("test", &attrs, &mut reg);
		result.is_ok().xpect_true();

		let container = result.unwrap();
		container.is_some().xpect_true();

		if let Some(Container::Struct(fields)) = container {
			// 'self' should be renamed to 'self_ref'
			fields
				.iter()
				.find(|f| f.name == "self_ref")
				.is_some()
				.xpect_true();

			// 'Self' should be renamed to 'Self_ref'
			fields
				.iter()
				.find(|f| f.name == "Self_ref")
				.is_some()
				.xpect_true();

			// other reserved words should use raw identifier
			fields
				.iter()
				.find(|f| f.name == "r#type")
				.is_some()
				.xpect_true();

			// should not contain raw identifiers for self/Self
			fields
				.iter()
				.find(|f| f.name == "r#self" || f.name == "r#Self")
				.is_none()
				.xpect_true();
		}
	}
}
