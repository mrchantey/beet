//! The BSX-template-by-name registry, resolving `<path::to::X>` tags.
//!
//! An uppercase tag resolves two ways: a Rust type via the
//! [`ReflectTemplate`](beet_core::prelude::ReflectTemplate) bridge (a registered
//! `#[template]` or `#[reflect(Component)]` type), or a `.bsx`-authored template
//! registered here by its module path.
//!
//! Templates are named by file path as modules, Rust-style: `<path::to::X>`
//! resolves to `path/to/X.bsx`. [`BsxTemplateRegistry::insert_source_from_path`]
//! indexes one `(path, source)` pair into a name -> (schema, parsed tree)
//! registry, the manifest a reactive client layer also reads; the reactive
//! [`TemplateDir`](crate::prelude::TemplateDir) loader reads a directory through a
//! `BlobStore` and hands each pair here. The source format is pluggable per file
//! type via [`TemplateFormats`]: `.bsx` parses through the markup grammar, `.js`
//! wraps in a `<script>`. A `<script type="json" bx:schema>` block in a `.bsx`
//! declares the template's prop schema (see [`super::schema`]).

use super::ast::*;
use crate::prelude::*;

/// An in-memory registry mapping a BSX template's module path (eg
/// `path::to::X`) to its parsed syntax tree and optional prop schema.
///
/// A `<path::to::X>` tag resolves here when no Rust type is registered under
/// that name. The stored tree is built into the calling entity at resolution,
/// carrying its own slot markers so the walker composes caller content into it.
#[derive(Default, Resource, Clone)]
pub struct BsxTemplateRegistry {
	templates: HashMap<SmolStr, BsxTemplateDef>,
}

/// A registered BSX template: its parsed root nodes and optional prop schema.
#[derive(Clone)]
pub struct BsxTemplateDef {
	/// The template body's root nodes, built into the calling entity.
	pub nodes: Vec<BsxNode>,
	/// The inline prop schema declared by a `<script bx:schema>` block, if any.
	/// Validated against a tag's props at build time, like a Rust template's.
	pub schema: Option<ValueSchema>,
	/// A remote schema URL declared by `<script bx:schema src="..">`, resolved
	/// asynchronously and awaited by `LoadTemplate`.
	pub remote_schema: Option<SmolStr>,
}

impl BsxTemplateRegistry {
	/// Register a template under `name` from already-parsed `nodes`, extracting a
	/// `bx:schema` block (inline or remote) as its schema.
	pub fn insert(&mut self, name: impl Into<SmolStr>, nodes: Vec<BsxNode>) {
		let (schema, remote_schema) =
			match super::schema::extract_schema_directive(&nodes) {
				super::schema::SchemaDirective::Inline(schema) => {
					(Some(schema), None)
				}
				super::schema::SchemaDirective::Remote(src) => {
					(None, Some(src))
				}
				super::schema::SchemaDirective::None => (None, None),
			};
		let nodes = super::schema::strip_schema_blocks(nodes);
		self.templates.insert(name.into(), BsxTemplateDef {
			nodes,
			schema,
			remote_schema,
		});
	}

	/// Parse `source` as a BSX template body and register it under `name`.
	pub fn insert_source(
		&mut self,
		name: impl Into<SmolStr>,
		source: &str,
	) -> Result {
		let nodes = super::parse::parse_document(
			source,
			&super::parse::BsxParseConfig::bsx(),
		)?;
		self.insert(name, nodes);
		Ok(())
	}

	/// Parse `source` through the format `formats` registers for its [`MediaType`]
	/// and register it under the module path derived from `path`, relative to a
	/// `templates/` root: `path/to/X.bsx` registers under `path::to::X`, so
	/// `<path::to::X>` resolves to it. A type with no registered format is skipped.
	///
	/// A caller reading a template directory through a `BlobStore` (the filesystem
	/// in dev, S3/R2 when deployed) hands each `(path, source)` pair here. Pure (no
	/// I/O), so it lives here rather than on the higher store layer; the reactive
	/// [`TemplateDir`](crate::prelude::TemplateDir) loader is the store-backed caller.
	pub fn insert_source_from_path(
		&mut self,
		formats: &TemplateFormats,
		path: &SmolPath,
		source: &str,
	) -> Result {
		// a type with no registered format is skipped.
		let Some(parse) = path.media_type().and_then(|ty| formats.get(&ty))
		else {
			return Ok(());
		};
		let module = module_path_from_rel(path).ok_or_else(|| {
			bevyhow!("could not derive a module path from `{path}`")
		})?;
		self.insert(module, parse(source)?);
		Ok(())
	}

	/// Look up a registered template by its module path.
	pub fn get(&self, name: &str) -> Option<&BsxTemplateDef> {
		self.templates.get(name)
	}

	/// The schema registered for the template under `name`, if any.
	pub fn schema(&self, name: &str) -> Option<&ValueSchema> {
		self.templates.get(name).and_then(|def| def.schema.as_ref())
	}

	/// Whether a template is registered under `name`.
	pub fn contains(&self, name: &str) -> bool {
		self.templates.contains_key(name)
	}

	/// The registered template names paired with their schemas, the manifest the
	/// reactive client layer reads and the source for [`SchemaRegistry`]
	/// population.
	pub fn manifest(
		&self,
	) -> impl Iterator<Item = (&SmolStr, Option<&ValueSchema>)> {
		self.templates
			.iter()
			.map(|(name, def)| (name, def.schema.as_ref()))
	}

	/// Mirror the schemas of the world's [`BsxTemplateRegistry`] into the
	/// [`SchemaRegistry`], so a composable [`ValueSchema::Reference`] between BSX
	/// templates resolves. Called after any template registration (eg the reactive
	/// [`TemplateDir`](crate::prelude::TemplateDir) loader). Wasm-safe (no I/O).
	pub fn refresh_schemas(world: &mut World) {
		let Some(registry) = world.get_resource::<BsxTemplateRegistry>() else {
			return;
		};
		let schemas = registry
			.manifest()
			.filter_map(|(name, schema)| {
				schema.map(|schema| (name.clone(), schema.clone()))
			})
			.collect::<Vec<_>>();
		let mut schema_registry = world.get_resource_or_init::<SchemaRegistry>();
		for (name, schema) in schemas {
			schema_registry.insert(name, schema);
		}
	}
}

/// The `::`-joined module path of a `.bsx` template at `path`, relative to a
/// template-dir root: `path/to/X.bsx` -> `path::to::X`. Store-backed (operates on
/// a store-relative [`SmolPath`], no filesystem).
fn module_path_from_rel(path: &SmolPath) -> Option<String> {
	let mut segments = path.segments();
	let stem = path.file_stem()?;
	*segments.last_mut()? = stem;
	(!segments.is_empty()).then(|| segments.join("::"))
}

#[cfg(all(test, feature = "fs", not(target_arch = "wasm32")))]
mod test {
	use super::*;

	#[beet_core::test]
	fn module_path_from_rel_derives_module() {
		module_path_from_rel(&SmolPath::from("path/to/X.bsx"))
			.unwrap()
			.xpect_eq("path::to::X".to_string());
		module_path_from_rel(&SmolPath::from("Todo.bsx"))
			.unwrap()
			.xpect_eq("Todo".to_string());
	}
}
