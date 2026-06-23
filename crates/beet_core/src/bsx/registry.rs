//! The BSX-template-by-name registry, resolving `<path::to::X>` tags.
//!
//! An uppercase tag resolves two ways: a Rust type via the
//! [`ReflectTemplate`](beet_core::prelude::ReflectTemplate) bridge (a registered
//! `#[template]` or `#[reflect(Component)]` type), or a `.bsx`-authored template
//! registered here by its module path.
//!
//! Templates are named by file path as modules, Rust-style: `<path::to::X>`
//! resolves to `path/to/X.bsx`. [`BsxTemplateRegistry::register_dir`] indexes a
//! directory into a name -> (schema, parsed tree) registry, the manifest a
//! reactive client layer also reads. The source format is pluggable per file
//! type via [`TemplateFormats`]: `.bsx` parses through the markup grammar, `.js`
//! wraps in a `<script>`. A `<script type="json" bx:schema>` block in a `.bsx`
//! declares the template's prop schema (see [`super::schema`]).

use super::ast::*;
use crate::prelude::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use std::path::Path;

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
	/// The store-backed counterpart to [`register_dir`](Self::register_dir): a
	/// caller reading `templates/` through a `BlobStore` (the filesystem in dev, S3
	/// when deployed) hands each `(path, source)` pair here. Pure (no I/O), so it
	/// lives here rather than on the higher store layer.
	pub fn insert_source_from_path(
		&mut self,
		formats: &TemplateFormats,
		path: &SmolPath,
		source: &str,
	) -> Result {
		// a type with no registered format is skipped, mirroring `register_dir`.
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

	/// Index a directory of template source files, registering each by its module
	/// path.
	///
	/// A file `<dir>/path/to/X.bsx` registers under the module path `path::to::X`,
	/// so `<path::to::X>` resolves to it; a `path/to/X.js` registers the same name,
	/// its body the `<script>` wrapper. The file's [`MediaType`] selects its parser
	/// from `formats`, and a type with no registered format is skipped. This is the
	/// registration pass: all templates are registered before any are loaded, so a
	/// tag resolves to a known template and its schema. Uses `fs_ext`
	/// (cross-platform).
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub fn register_dir(
		&mut self,
		formats: &TemplateFormats,
		dir: impl AsRef<Path>,
	) -> Result {
		let dir = dir.as_ref();
		for path in ReadDir::files_recursive(dir)? {
			let Some(parse) = path.media_type().and_then(|ty| formats.get(&ty))
			else {
				continue;
			};
			let module = module_path_of(dir, &path)?;
			let nodes = parse(&fs_ext::read_to_string(&path)?)?;
			self.insert(module, nodes);
		}
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
}

/// Filesystem registration of a BSX template directory. Native-only: it scans a
/// `dir` off disk. The in-memory schema mirror lives on [`WorldRegisterBsxSchemasExt`]
/// so it stays available on wasm.
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
#[extend::ext(name=WorldRegisterBsxExt)]
pub impl World {
	/// Register every template source under `dir` by its module path, populating the
	/// [`BsxTemplateRegistry`] and copying each template's prop schema into the
	/// [`SchemaRegistry`] so a composable [`ValueSchema::Reference`] resolves. The
	/// recognized formats are the world's [`TemplateFormats`] (`.bsx` and `.js` by
	/// default).
	///
	/// This is the registration pass: all templates are registered before any are
	/// loaded, so a tag resolves to a known template and its schema and a missing
	/// required field is a real error.
	fn register_bsx_templates(&mut self, dir: impl AsRef<Path>) -> Result {
		let formats = self.get_resource_or_init::<TemplateFormats>().clone();
		let mut registry = self
			.remove_resource::<BsxTemplateRegistry>()
			.unwrap_or_default();
		registry.register_dir(&formats, dir)?;
		self.insert_resource(registry);
		self.register_bsx_schemas();
		Ok(())
	}
}

/// The in-memory schema mirror, split out so it builds on wasm (it touches no
/// filesystem). The store-backed registration paths (eg a Cloudflare Worker
/// reading `templates/` from R2) call this after `insert_source`.
#[extend::ext(name=WorldRegisterBsxSchemasExt)]
pub impl World {
	/// Mirror the schemas of the already-populated [`BsxTemplateRegistry`] into the
	/// [`SchemaRegistry`], so a composable [`ValueSchema::Reference`] between BSX
	/// templates resolves. Used when templates were registered via
	/// [`BsxTemplateRegistry::insert_source`] rather than `register_bsx_templates`.
	fn register_bsx_schemas(&mut self) -> &mut Self {
		let Some(registry) = self.get_resource::<BsxTemplateRegistry>() else {
			return self;
		};
		let schemas = registry
			.manifest()
			.filter_map(|(name, schema)| {
				schema.map(|schema| (name.clone(), schema.clone()))
			})
			.collect::<Vec<_>>();
		let mut schema_registry = self.get_resource_or_init::<SchemaRegistry>();
		for (name, schema) in schemas {
			schema_registry.insert(name, schema);
		}
		self
	}
}

/// The Rust-style module path of a `.bsx` file relative to its template `dir`:
/// `<dir>/path/to/X.bsx` -> `path::to::X`.
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
fn module_path_of(dir: &Path, path: &Path) -> Result<String> {
	let relative = path.strip_prefix(dir).map_err(|_| {
		bevyhow!(
			"template file `{}` is not under `{}`",
			path.display(),
			dir.display()
		)
	})?;
	let without_ext = relative.with_extension("");
	let module = without_ext
		.components()
		.filter_map(|component| component.as_os_str().to_str())
		.collect::<Vec<_>>()
		.join("::");
	if module.is_empty() {
		bevybail!("could not derive a module path from `{}`", path.display());
	}
	Ok(module)
}

/// The `::`-joined module path of a `.bsx` template at `path`, relative to a
/// `templates/` root: `path/to/X.bsx` -> `path::to::X`. The store-backed
/// counterpart to [`module_path_of`], which strips a filesystem `dir` prefix.
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

	#[beet_core::test]
	fn module_path_from_file() {
		let dir = Path::new("/templates");
		let path = Path::new("/templates/path/to/X.bsx");
		module_path_of(dir, path)
			.unwrap()
			.xpect_eq("path::to::X".to_string());
	}

	#[beet_core::test]
	fn module_path_top_level() {
		let dir = Path::new("/templates");
		let path = Path::new("/templates/Todo.bsx");
		module_path_of(dir, path)
			.unwrap()
			.xpect_eq("Todo".to_string());
	}
}
