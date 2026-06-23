//! The cross-platform site build core shared by the native `beet` binary, the wasm
//! Worker entry, and the `check`/`export-static` commands.
//!
//! A site load splits into a world-free async read ([`read_site_sources`]: the site
//! `templates/` and the entry document, through the [`BlobStore`]) and a synchronous
//! world build ([`build_site_root`]: register the templates, build the entry into a
//! root carrying the store). The same path runs on the native async runtime and the
//! single-threaded wasm Worker, so entry resolution comes from an injected store
//! rather than a filesystem walk.

use beet::prelude::*;

/// The site sources read from a store: the `templates/` `(path, source)` pairs and
/// the entry document bytes, plus the entry name and the formats they register
/// through. The world-free async read result [`build_site_root`] consumes.
pub struct SiteSources {
	entry_name: String,
	entry: MediaBytes,
	templates: Vec<(SmolPath, String)>,
	formats: TemplateFormats,
}

/// Read the site `templates/` and the entry document through `store`, awaited off
/// the runtime (never blocked, so it runs on the single-threaded Worker too). The
/// caller reads `formats` from the world first, since the read itself is world-free.
pub async fn read_site_sources(
	store: &BlobStore,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
) -> Result<SiteSources> {
	let entry_name = entry_name.into();
	let templates =
		read_site_templates(store, &formats, &SmolPath::from(DEFAULT_TEMPLATES_DIR))
			.await?;
	let entry = store.get_media(&SmolPath::from(entry_name.as_str())).await?;
	SiteSources {
		entry_name,
		entry,
		templates,
		formats,
	}
	.xok()
}

/// Build read [`SiteSources`] into a root carrying `store` (resolved by ancestry for
/// `<RoutesDir>` and `<Template src>`), with `extra` riding onto the root (eg
/// `DisableBootOnLoad` for a render-only build). The synchronous world-mutating tail
/// of a site load; returns the root entity.
pub fn build_site_root(
	world: &mut World,
	store: BlobStore,
	sources: SiteSources,
	extra: impl Bundle,
) -> Result<Entity> {
	let SiteSources {
		entry_name,
		entry,
		templates,
		formats,
	} = sources;
	register_site_templates(world, &formats, templates)?;
	let template = EntryTemplate::from_bytes(world, &entry)
		.map_err(|err| bevyhow!("failed to parse entry `{entry_name}`: {err}"))?;
	// the site store on the root: descendants resolve it by ancestry.
	let root = world.spawn((extra, store)).id();
	world
		.entity_mut(root)
		.insert_template(template)
		.map_err(|err| bevyhow!("failed to load entry `{entry_name}`: {err}"))?;
	world.flush();
	Ok(root)
}

#[cfg(test)]
mod test {
	use super::*;

	/// The shared core builds an entry from any store: an in-memory store here, so
	/// it runs storage-agnostic (on wasm too), no filesystem involved. The entry's
	/// `<DefaultAppRoutes/>` lands on the built router root.
	#[beet::test]
	async fn builds_an_entry_from_an_in_memory_store() {
		let store = BlobStore::temp();
		store
			.insert(
				&SmolPath::from("main.bsx"),
				"<Router><DefaultAppRoutes/></Router>",
			)
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let sources =
			read_site_sources(&store, formats, "main.bsx").await.unwrap();
		let root =
			build_site_root(&mut world, store, sources, DisableBootOnLoad).unwrap();
		// the entry built into a router root carrying the default app routes
		world.entity(root).contains::<Router>().xpect_true();
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["js", "reactivity.js"])
			.xpect_some();
	}
}
