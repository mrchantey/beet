//! The site root: the [`BlobStore`] that site-relative paths resolve against.
//!
//! Backs the native directory-scanning [`RoutesDir`](crate::prelude::RoutesDir),
//! the cross-platform `<Template src>` include, and the entry document plus
//! `templates/` load. Provider-agnostic: local dev roots it at an [`FsStore`], a
//! deployed task at an `S3Store`, both behind the same [`BlobStore`], so the same
//! site loads from disk or from S3 unchanged.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// The [`BlobStore`] site-relative paths resolve against: the `<RoutesDir src>`
/// scan root, the `<Template src>` include base, and the source the entry document
/// and `templates/` load from. A host sets this to a store rooted at its entry's
/// project directory; it defaults to an [`FsStore`] at the workspace root.
#[derive(Debug, Clone, Resource)]
pub struct SiteRoot(pub BlobStore);

impl SiteRoot {
	/// A site root backed by an [`FsStore`] rooted at `dir`.
	#[cfg(feature = "std")]
	pub fn new_fs(dir: impl Into<AbsPathBuf>) -> Self {
		Self(BlobStore::new(FsStore::new(dir)))
	}

	/// A site root backed by an [`FsStore`] at `path`, workspace-relative.
	#[cfg(feature = "std")]
	pub fn new_workspace_rel(
		path: impl AsRef<std::path::Path>,
	) -> FsResult<Self> {
		AbsPathBuf::new_workspace_rel(path).map(Self::new_fs)
	}

	/// Register every `.bsx` template under this site store's `templates/`
	/// directory, mirroring
	/// [`register_bsx_templates`](beet_core::prelude::WorldRegisterBsxExt::register_bsx_templates)
	/// but reading through the [`BlobStore`] (the filesystem in dev, S3 when
	/// deployed) rather than scanning the local filesystem.
	///
	/// Native blocking entry point; wasm (where blocking would hang) uses
	/// [`register_templates_async`](Self::register_templates_async).
	#[cfg(all(feature = "bsx", not(target_arch = "wasm32")))]
	pub fn register_templates(&self, world: &mut World) -> Result {
		// read every `.bsx` source through the store, then register them.
		let sources = async_ext::block_on(self.read_templates())?;
		register_template_sources(world, sources)
	}

	/// The async core of [`register_templates`](Self::register_templates):
	/// register every `.bsx` template under `templates/`, reading through the
	/// store without blocking. The wasm site-load path (the Cloudflare Worker)
	/// awaits this instead of blocking the runtime.
	#[cfg(feature = "bsx")]
	pub async fn register_templates_async(&self, world: &mut World) -> Result {
		let sources = self.read_templates().await?;
		register_template_sources(world, sources)
	}

	/// List and read every `.bsx` source under this site store's `templates/`
	/// directory as `(path, source)` pairs. Shared by the native and wasm
	/// registration paths.
	#[cfg(feature = "bsx")]
	async fn read_templates(&self) -> Result<Vec<(SmolPath, String)>> {
		let store = self.0.with_subdir(SmolPath::from("templates"));
		// no `templates/` dir (eg a single-file entry): nothing to register.
		if !store.store_exists().await? {
			return Ok(Vec::new());
		}
		store
			.list()
			.await?
			.into_iter()
			.filter(|path| path.extension() == Some("bsx"))
			.map(async |path| {
				let bytes = store.get(&path).await?;
				Ok((path, String::from_utf8(bytes.to_vec())?))
			})
			.xmap(async_ext::try_join_all)
			.await
	}
}

/// Register each `(path, source)` `.bsx` template into the world's
/// [`BsxTemplateRegistry`] by its module path, then refresh the BSX schemas.
/// The world-mutating tail shared by the native and wasm registration paths.
#[cfg(feature = "bsx")]
fn register_template_sources(
	world: &mut World,
	sources: Vec<(SmolPath, String)>,
) -> Result {
	let mut registry = world
		.remove_resource::<BsxTemplateRegistry>()
		.unwrap_or_default();
	for (path, source) in sources {
		let module = module_path_of(&path).ok_or_else(|| {
			bevyhow!("could not derive a module path from `{path}`")
		})?;
		registry.insert_source(module, &source)?;
	}
	world.insert_resource(registry);
	world.register_bsx_schemas();
	Ok(())
}

impl Default for SiteRoot {
	fn default() -> Self {
		cfg_if! {
			if #[cfg(feature = "std")] {
				Self(BlobStore::new(FsStore::default()))
			} else {
				Self(BlobStore::temp())
			}
		}
	}
}

/// The `::`-joined module path of a `.bsx` template at `path` (relative to the
/// `templates/` store root): `path/to/X.bsx` -> `path::to::X`.
#[cfg(feature = "bsx")]
fn module_path_of(path: &SmolPath) -> Option<String> {
	let mut segments = path.segments();
	let stem = path.file_stem()?;
	*segments.last_mut()? = stem;
	(!segments.is_empty()).then(|| segments.join("::"))
}
