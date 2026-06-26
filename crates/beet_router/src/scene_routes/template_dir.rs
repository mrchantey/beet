//! Runtime template registration: a directory of `.bsx`/`.js` templates becomes
//! resolvable `<path::to::X>` tags at spawn time, no codegen.
//!
//! Inserting a [`TemplateDir`] (eg from a `main.bsx` entry via
//! `<TemplateDir src="templates"/>`) triggers [`TemplateDir::register_on_insert`]:
//! the nearest ancestor [`BlobStore`] is scoped to `src`, every recognized
//! template source under it is read and registered into the
//! [`BsxTemplateRegistry`] by its module path (`templates/widgets/Card.bsx` ->
//! `widgets::Card`), and the BSX schemas are refreshed. Store-backed, so it reads
//! identically from the local filesystem in dev, S3 in a deployed task, R2 in a
//! Worker, or an embedded in-memory store a library crate ships.
//!
//! An entry's *own* markup may reference a template at parse time (eg `<Styles/>`),
//! which must resolve before the entry builds. That case is handled by reading the
//! entry's declared dirs and registering them synchronously *before* the entry
//! parses (see [`TemplateDir::extract_dirs`] and the cli's `build_entry_root`); the
//! reactive observer covers everything that resolves later (route pages, library
//! widgets, live reload).

use beet_core::prelude::*;
use beet_net::prelude::*;

/// Declares a directory of `.bsx`/`.js` templates, relative to the nearest
/// ancestor [`BlobStore`], registering each as a `<path::to::X>` tag (see the
/// module docs).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct TemplateDir {
	/// The template directory, relative to the nearest ancestor [`BlobStore`].
	pub src: String,
}

impl TemplateDir {
	/// Register templates under `src`, relative to the nearest ancestor [`BlobStore`].
	pub fn new(src: impl Into<String>) -> Self { Self { src: src.into() } }

	/// Observer: read the [`TemplateDir`]'s store and register its templates (see
	/// the module docs).
	///
	/// The read is store I/O (the filesystem in dev, S3/R2 when deployed), so it
	/// runs as an [`AsyncEntity`] task rather than blocking the runtime (which is
	/// single-threaded on wasm). The nearest ancestor [`BlobStore`] is resolved
	/// *inside* that task, where the whole tree is built, so the ancestor link is
	/// present; a store-less app is an error. On completion the entity is marked
	/// [`TemplatesLoaded`] so a boot path can [`settle_all`](RoutesDir::settle_all).
	pub fn register_on_insert(
		ev: On<Insert, TemplateDir>,
		dirs: Query<&TemplateDir>,
		formats: Res<TemplateFormats>,
		mut commands: Commands,
	) -> Result {
		let entity = ev.entity;
		let src = dirs.get(entity)?.src.clone();
		let formats = formats.clone();
		// `queue_async_local` (not `queue_async`): the read is bridge-heavy (resolve the
		// ancestor store, then register the sources back on the world), and the async
		// bridge only guarantees a bridge poll completes when the task runs on the
		// runtime's local executor. A `bevy_multithreaded` build's `spawn` would run it on
		// a worker thread whose bridge poll can miss the main-thread world-scope window and
		// stall the registration, leaving the dir un-`TemplatesLoaded`. Local keeps it
		// deterministic on every target.
		commands.entity(entity).queue_async_local(
			async move |dir: AsyncEntity| -> Result {
				let store = dir
					.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
						|entity, stores| {
							stores.get(entity).map(BlobStore::clone)
						},
					)
					.await??;
				let sources =
					Self::read_sources(&store, &src, &formats).await?;
				dir.world()
					.with(move |world| -> Result {
						Self::register_sources(world, &formats, sources)?;
						world.entity_mut(entity).insert(TemplatesLoaded);
						world.flush();
						Ok(())
					})
					.await?;
				Ok(())
			},
		);
		Ok(())
	}

	/// Read every recognized template source under the store's `src` subdirectory
	/// as `(path, source)` pairs (each path relative to `src`), keeping only files
	/// whose [`MediaType`] `formats` recognizes (`.bsx`, `.js`). Async (store I/O),
	/// so a load path awaits it off the runtime. A missing directory yields no
	/// pairs, so an entry can declare a dir it does not ship.
	pub async fn read_sources(
		store: &BlobStore,
		src: &str,
		formats: &TemplateFormats,
	) -> Result<Vec<(SmolPath, String)>> {
		let store = store.with_subdir(SmolPath::from(src));
		if !store.store_exists().await? {
			return Ok(Vec::new());
		}
		store
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
			.await
	}

	/// Register each `(path, source)` pair into the world's [`BsxTemplateRegistry`]
	/// by its module path, lowering each through the format its [`MediaType`]
	/// selects, then refresh the BSX schemas. The synchronous world-mutating tail of
	/// a template-dir load, applied once [`read_sources`](Self::read_sources)
	/// resolves. Additive: existing registrations are kept, so multiple dirs
	/// accumulate.
	pub fn register_sources(
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
		BsxTemplateRegistry::refresh_schemas(world);
		Ok(())
	}

	/// Collect the `src` of every `<TemplateDir>` element in a parsed entry tree,
	/// the registry-free pre-scan the cli runs to register an entry's own template
	/// dirs before the entry parses (so entry-level tags like `<Styles/>` resolve).
	pub fn extract_dirs(nodes: &[BsxNode]) -> Vec<String> {
		let mut dirs = Vec::new();
		Self::collect_dirs(nodes, &mut dirs);
		dirs
	}

	/// Recursively collect `<TemplateDir src=..>` declarations from `nodes`.
	fn collect_dirs(nodes: &[BsxNode], dirs: &mut Vec<String>) {
		for node in nodes {
			let BsxNode::Element(element) = node else {
				continue;
			};
			if element.tag == "TemplateDir"
				&& let Some(src) = Self::element_src(element)
			{
				dirs.push(src);
			}
			Self::collect_dirs(&element.children, dirs);
		}
	}

	/// The string value of an element's `src` attribute, if present.
	fn element_src(element: &BsxElement) -> Option<String> {
		element
			.attributes
			.iter()
			.find(|attr| attr.key == "src")
			.and_then(|attr| match &attr.value {
				AttrValue::Str(src) => Some(src.clone()),
				_ => None,
			})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// An in-memory [`BlobStore`] seeded with `files`, so registration is
	/// provider-agnostic (runs on wasm too).
	async fn memory_fixture(files: &[(&str, &str)]) -> BlobStore {
		let store = BlobStore::temp();
		for (rel, content) in files {
			store
				.insert(&SmolPath::from(*rel), content.to_string())
				.await
				.unwrap();
		}
		store
	}

	/// `extract_dirs` finds every `<TemplateDir>` src, at any depth, from a parsed
	/// entry tree.
	#[beet_core::test]
	fn extract_dirs_walks_tree() {
		let nodes = parse_document(
			"<Router><TemplateDir src=\"templates\"/><div><TemplateDir src=\"more\"/></div></Router>",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		TemplateDir::extract_dirs(&nodes)
			.xpect_eq(vec!["templates".to_string(), "more".to_string()]);
	}

	/// Inserting a [`TemplateDir`] over a store registers its templates so a
	/// `<widgets::Card>` tag resolves, store-agnostic (wasm too).
	#[beet_core::test]
	async fn registers_templates_from_store() {
		let mut world = router_world();
		let store = memory_fixture(&[(
			"templates/widgets/Card.bsx",
			"<section class=\"card\"><Slot/></section>",
		)])
		.await;
		world.spawn((store, TemplateDir::new("templates")));
		AsyncRunner::settle_async_tasks(&mut world).await;
		world
			.resource::<BsxTemplateRegistry>()
			.contains("widgets::Card")
			.xpect_true();
	}
}
