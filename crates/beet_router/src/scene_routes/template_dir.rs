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
//! parses (see [`EntryPrescan`] and the cli's entry build); the reactive observer
//! covers everything that resolves later (route pages, library widgets, live
//! reload).

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

/// The template names an owner (a [`TemplateDir`] entity, or the entry root for
/// the entry-level pre-registration) registered into the
/// [`BsxTemplateRegistry`]. Re-registering diffs against it so deleted sources
/// unregister; despawning the owner unregisters everything it owned (via the
/// `on_remove` hook), so a torn-down entry scene leaves no stale templates.
#[derive(Debug, Default, Clone, Deref, Component)]
#[component(on_remove = RegisteredTemplates::on_remove())]
struct RegisteredTemplates(Vec<SmolStr>);

impl RegisteredTemplates {
	/// The `on_remove` hook: snapshot the owned names (still present during the
	/// hook), then unregister them via a queued command. Fires on removal and
	/// despawn, not on a replace (the re-register diff owns that path).
	fn on_remove() -> impl FnOnce(DeferredWorld, HookContext) {
		|mut world, cx| {
			let Some(names) =
				world.get::<RegisteredTemplates>(cx.entity).cloned()
			else {
				return;
			};
			world.commands().queue(move |world: &mut World| {
				let Some(mut registry) =
					world.get_resource_mut::<BsxTemplateRegistry>()
				else {
					return;
				};
				names.iter().for_each(|name| {
					registry.remove(name);
				});
			});
		}
	}
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
	/// present; a store-less app is an error. The registration parks a
	/// [`PendingGuard`] on the build root (or this entity outside a build), so a
	/// load or settle ([`TemplatePending::settle`]) waits for it; on completion
	/// the entity is also marked [`TemplatesLoaded`].
	pub fn register_on_insert(
		ev: On<Insert, TemplateDir>,
		dirs: Query<&TemplateDir>,
		formats: Res<TemplateFormats>,
		build_root: Option<Res<TemplateBuildRoot>>,
		mut commands: Commands,
	) -> Result {
		let entity = ev.entity;
		let src = dirs.get(entity)?.src.clone();
		let formats = formats.clone();
		let root = build_root.map(|root| **root);
		// one queued command parks the guard (ahead of the build's synchronous
		// drain) and spawns the read task holding it, so however the task ends the
		// guard resolves.
		//
		// `run_async_local` (not `run_async`): the read is bridge-heavy (resolve the
		// ancestor store, then register the sources back on the world), and the async
		// bridge only guarantees a bridge poll completes when the task runs on the
		// runtime's local executor. A `bevy_multithreaded` build's `spawn` would run it on
		// a worker thread whose bridge poll can miss the main-thread world-scope window and
		// stall the registration, leaving the dir un-`TemplatesLoaded`. Local keeps it
		// deterministic on every target.
		commands.queue(move |world: &mut World| {
			let guard = TemplatePending::park_on(
				world,
				root.unwrap_or(entity),
				PendingKind::Passive,
				format!("<TemplateDir src=\"{src}\"> read"),
			);
			let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
				// the dir despawned before the command ran: the dropped guard
				// resolves through the sweep.
				return;
			};
			entity_mut.run_async_local(
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
							Self::register_sources(
								world, entity, &formats, sources,
							)?;
							// watch the templates dir for live reload (keyed to its base
							// store); inert on a non-fs store / on wasm.
							let scoped =
								store.with_subdir(SmolPath::from(src.as_str()));
							{
								let mut entity_mut = world.entity_mut(entity);
								entity_mut.insert(TemplatesLoaded);
								if let Some(watch) =
									WatchDir::from_store(&scoped)
								{
									entity_mut.insert(watch);
								}
							}
							world.flush();
							guard.resolve(world);
							Ok(())
						})
						.await?;
					Ok(())
				},
			);
		});
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
	/// resolves.
	///
	/// Registrations are *owned* by `owner` (the `TemplateDir` entity, or the entry
	/// root for the entry-level pre-registration): the owner's previous set is
	/// diffed so a source it no longer ships unregisters (a deleted template on
	/// live reload), and despawning the owner unregisters everything it owned (the
	/// structural teardown+rebuild path). Distinct owners still accumulate, so
	/// multiple dirs compose.
	pub fn register_sources(
		world: &mut World,
		owner: Entity,
		formats: &TemplateFormats,
		sources: Vec<(SmolPath, String)>,
	) -> Result {
		let mut registry = world
			.remove_resource::<BsxTemplateRegistry>()
			.unwrap_or_default();
		let mut names = Vec::new();
		for (path, source) in sources {
			names.extend(
				registry.insert_source_from_path(formats, &path, &source)?,
			);
		}
		// unregister sources this owner previously registered but no longer ships.
		if let Some(previous) = world.get::<RegisteredTemplates>(owner) {
			previous
				.iter()
				.filter(|name| !names.contains(name))
				.cloned()
				.collect::<Vec<_>>()
				.into_iter()
				.for_each(|stale| {
					registry.remove(&stale);
				});
		}
		world.insert_resource(registry);
		if let Ok(mut owner) = world.get_entity_mut(owner) {
			owner.insert(RegisteredTemplates(names));
		}
		BsxTemplateRegistry::refresh_schemas(world);
		Ok(())
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
