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
pub struct Schema {
	provider: Provider,
	data_source_schemas: Option<BTreeMap<String, SchemaItem>>,
	resource_schemas: Option<BTreeMap<String, SchemaItem>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Provider {
	version: i64,
	block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SchemaItem {
	version: i64,
	block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Block {
	attributes: Option<BTreeMap<String, Attribute>>,
	block_types: Option<BTreeMap<String, NestedBlock>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StringKind {
	Plain,
	Markdown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Attribute {
	r#type: Option<AttributeType>,
	description: Option<String>,
	required: Option<bool>,
	optional: Option<bool>,
	computed: Option<bool>,
	sensitive: Option<bool>,
	description_kind: Option<StringKind>,
	deprecated: Option<bool>,
	/// Present when the attribute uses an inline structural type instead of `type`.
	nested_type: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NestedBlock {
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
	filter: &ResourceFilter,
	config: &CodeGeneratorConfig,
) -> Result<(Registry, Vec<ResourceMeta>, DocComments)> {
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

				meta.push(ResourceMeta {
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
		for (attr_name, attr) in attrs {
			if let Some(desc) = &attr.description {
				if !desc.is_empty() {
					let key = vec![
						module_name.to_string(),
						container_name.to_string(),
						attr_name.clone(),
					];
					comments.insert(key, desc.clone());
				}
			}
		}
	}

	if let Some(block_types) = &block.block_types {
		for (bt_name, nested) in block_types {
			// Build the nested container name the same way export_block_type does.
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

fn export_attributes(
	attrs: &BTreeMap<String, Attribute>,
) -> Result<Option<Container>> {
	let mut target_attrs = Vec::new();
	for (an, at) in attrs {
		let an = RESERVED_WORDS
			.iter()
			.find(|word| an == &word.to_string())
			.map(|word| format!("r#{}", word))
			.unwrap_or_else(|| an.to_string());

		// When `type` is absent the attribute uses `nested_type` (an inline
		// structural definition).  Treat it as a map of strings for now.
		let field_type = match &at.r#type {
			Some(AttributeType(Value::String(type_str)))
				if type_str == "string" =>
			{
				FieldType::Str
			}
			Some(AttributeType(Value::String(type_str)))
				if type_str == "bool" =>
			{
				FieldType::Bool
			}
			Some(AttributeType(Value::String(type_str)))
				if type_str == "number" =>
			{
				FieldType::I64
			}
			Some(AttributeType(Value::String(type_str)))
				if type_str == "set" || type_str == "list" =>
			{
				FieldType::Seq(Box::new(FieldType::Str))
			}
			Some(AttributeType(Value::String(type_str)))
				if type_str == "map" =>
			{
				FieldType::Map {
					key: Box::new(FieldType::Str),
					value: Box::new(FieldType::Str),
				}
			}
			// Terraform "dynamic" pseudo-type can hold any value; approximate as String.
			Some(AttributeType(Value::String(type_str)))
				if type_str == "dynamic" =>
			{
				FieldType::Str
			}
			Some(AttributeType(Value::String(type_str))) => {
				bevybail!("Unknown type {}", type_str);
			}
			Some(AttributeType(Value::Array(type_arr)))
				if type_arr.first().unwrap() == "set"
					|| type_arr.first().unwrap() == "list" =>
			{
				FieldType::Seq(Box::new(FieldType::Str))
			}
			/* TODO: It will assume a map of strings even if the specified type is of a different kind (e.g. map of object) */
			Some(AttributeType(Value::Array(type_arr)))
				if type_arr.first().unwrap() == "map" =>
			{
				FieldType::Map {
					key: Box::new(FieldType::Str),
					value: Box::new(FieldType::Str),
				}
			}
			// ["object", {...}], ["tuple", [...]], or any other complex array
			// type spec we don't fully model yet — approximate as a map of
			// strings so code generation can proceed.
			Some(AttributeType(Value::Array(_))) => FieldType::Map {
				key: Box::new(FieldType::Str),
				value: Box::new(FieldType::Str),
			},
			Some(unknown) => {
				bevybail!("Type {:?} not supported", unknown);
			}
			// `nested_type` attribute — no top-level `type` field.
			// Approximate as a map of JSON strings for now.
			None => FieldType::Map {
				key: Box::new(FieldType::Str),
				value: Box::new(FieldType::Str),
			},
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

		target_attrs.push(Field::new(an, attr_fmt));
	}
	if !target_attrs.is_empty() {
		Ok(Some(Container::Struct(target_attrs)))
	} else {
		Ok(None)
	}
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
	let mut cf1 = export_attributes(blk.attributes.as_ref().unwrap())?;
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
		let mut nested_cf = export_attributes(attrs)?;
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

	#[test]
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

	#[test]
	fn generate_registry_from_schema() {
		let tf_schema = read_tf_schema_from_file(
			fixtures_dir().join("test-provider-schema.json"),
		);
		let registry = export_schema_to_registry(&tf_schema.as_ref().unwrap());

		registry.is_ok().xpect_true();
		registry.unwrap().len().xpect_eq(10);
	}

	#[beet_core::test]
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
    smol_str = "0.2"

    [workspace]
    "#,
		)
		.unwrap();
		fs_ext::create_dir_all(dir.as_ref().join("src")).unwrap();
		let source_path = dir.as_ref().join("src/lib.rs");
		let mut source_buf = Vec::new();
		generate_serde("test", &mut source_buf, &registry.unwrap()).unwrap();
		// Inject a `beet_core` stub so `beet_core::prelude::SmolStr` resolves
		// in the temp crate (which doesn't depend on the full beet_core).
		let generated = String::from_utf8(source_buf).unwrap();
		let beet_core_stub = concat!(
			"pub mod beet_core {\n",
			"    pub mod prelude {\n",
			"        pub use smol_str::SmolStr;\n",
			"    }\n",
			"}\n",
		);
		// Insert after the first line (the `#![allow(…)]` inner attribute).
		let source_with_stub = if let Some(nl) = generated.find('\n') {
			let (first_line, rest) = generated.split_at(nl + 1);
			format!("{}{}{}", first_line, beet_core_stub, rest)
		} else {
			format!("{}{}", beet_core_stub, generated)
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

	#[test]
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

	#[test]
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

	#[test]
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

	#[test]
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
}
