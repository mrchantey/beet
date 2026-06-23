//! Reading a site's `templates/` directory through a [`BlobStore`].
//!
//! The store-backed counterpart to
//! [`register_bsx_templates`](beet_core::prelude::WorldRegisterBsxExt::register_bsx_templates):
//! a site load reads every recognized template source under the store's
//! `templates/` subdirectory and registers each by its module path, lowering it
//! through the format its [`MediaType`] selects (`.bsx`, `.js`), so the same
//! templates resolve whether the
//! site loads from the local filesystem (dev) or S3 (a deployed task). Split into
//! an async read and a synchronous world-mutating apply, so a load path awaits the
//! read off the runtime and then applies it without ever blocking.

use beet_core::prelude::*;
use beet_net::prelude::*;

/// The default site subdirectory scanned for BSX/JS templates.
pub const DEFAULT_TEMPLATE_DIR: &str = "templates";

/// The site subdirectories scanned for BSX/JS templates, each relative to the
/// site store root and registered by the module path *relative to that
/// directory* (so `templates/widgets/Card.bsx` registers `widgets::Card`).
///
/// Defaults to a single `templates/` directory. The `BEET_TEMPLATE_DIRS` env var
/// overrides it with a comma-separated list, letting a no-code site keep its
/// source under `src/` (eg `BEET_TEMPLATE_DIRS=src/templates,src/controls`, where
/// `src/controls/PresentationControls.js` registers as `PresentationControls`).
/// Scanned in order, so a later directory's name wins a collision.
pub fn site_template_dirs() -> Vec<SmolPath> {
	env_ext::var("BEET_TEMPLATE_DIRS")
		.ok()
		.map(|raw| {
			raw.split(',')
				.map(str::trim)
				.filter(|dir| !dir.is_empty())
				.map(SmolPath::from)
				.collect::<Vec<_>>()
		})
		.filter(|dirs| !dirs.is_empty())
		.unwrap_or_else(|| vec![SmolPath::from(DEFAULT_TEMPLATE_DIR)])
}

/// Read every template source under the site store's template directories (see
/// [`site_template_dirs`]) as `(path, source)` pairs (each path relative to its
/// scan directory), keeping only files whose [`MediaType`] `formats` recognizes
/// (`.bsx`, `.js`). Async (store I/O), so a load path awaits it off the runtime
/// rather than blocking. A site with no template directory (eg a single-file
/// entry) yields no pairs.
pub async fn read_site_templates(
	store: &BlobStore,
	formats: &TemplateFormats,
) -> Result<Vec<(SmolPath, String)>> {
	let mut sources = Vec::new();
	for dir in site_template_dirs() {
		let store = store.with_subdir(dir);
		// a missing template dir is skipped, so a site can declare more dirs than
		// it ships.
		if !store.store_exists().await? {
			continue;
		}
		let dir_sources = store
			.list()
			.await?
			.into_iter()
			.filter(|path| {
				path.media_type().and_then(|ty| formats.get(&ty)).is_some()
			})
			.map(async |path| -> Result<(SmolPath, String)> {
				let bytes = store.get(&path).await?;
				Ok((path, String::from_utf8(bytes.to_vec())?))
			})
			.xmap(async_ext::try_join_all)
			.await?;
		sources.extend(dir_sources);
	}
	Ok(sources)
}

/// Register each `(path, source)` template into the world's
/// [`BsxTemplateRegistry`] by its module path, lowering each source through the
/// format `formats` registers for its [`MediaType`], then refresh the BSX schemas.
/// The synchronous world-mutating tail of a site-template load, applied once
/// [`read_site_templates`] resolves.
pub fn register_site_templates(
	world: &mut World,
	formats: &TemplateFormats,
	sources: Vec<(SmolPath, String)>,
) -> Result {
	let mut registry = world
		.remove_resource::<BsxTemplateRegistry>()
		.unwrap_or_default();
	for (path, source) in sources {
		registry.insert_source_from_path(formats, &path, &source)?;
	}
	world.insert_resource(registry);
	world.register_bsx_schemas();
	Ok(())
}
