//! Runtime template registration: a directory of `.bsx`/`.js` templates becomes
//! resolvable `<path::to::X>` tags at spawn time, no codegen.
//!
//! Inserting a [`TemplateDir`] (eg from a `main.bsx` entry via
//! `<TemplateDir src="templates"/>`) derives a [`DirPath`] scoping the nearest
//! ancestor [`BlobStore`] to `src` (which also registers the dir's [`WatchDir`])
//! and triggers [`TemplateDir::register_on_insert`]: every recognized template
//! source under the scoped store is read and registered into the
//! [`BsxTemplateRegistry`] by its module path (`templates/widgets/Card.bsx` ->
//! `widgets::Card`), the BSX schemas are refreshed, and each source gets a
//! [`TemplateFile`] child entity. That child is the file: its [`Blob`] changing
//! re-registers exactly that source ([`register_changed_template_files`]) and
//! its despawn unregisters what it registered, while a file created or removed
//! under the dir marks the dir's store changed and re-runs the scan
//! ([`rescan_changed_dirs`]). Store-backed, so it reads identically from the
//! local filesystem in dev, S3 in a deployed task, R2 in a Worker, or an
//! embedded in-memory store a library crate ships.
//!
//! An entry's *own* markup may reference a template at parse time (eg `<Styles/>`),
//! which must resolve before the entry builds. That case is handled by reading the
//! entry's declared dirs and registering them synchronously *before* the entry
//! parses (see [`EntryPrescan`] and the cli's entry build); the reactive observer
//! covers everything that resolves later (route pages, library widgets, live
//! reload).

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Declares a directory of `.bsx`/`.js` templates, relative to the nearest
/// ancestor [`BlobStore`], registering each as a `<path::to::X>` tag (see the
/// module docs).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::component_hook(|dir: &TemplateDir| DirPath::derive(&dir.src)))]
pub struct TemplateDir {
	/// The template directory, relative to the nearest ancestor [`BlobStore`].
	pub src: RelPath,
}

/// The template names an owner (a [`TemplateFile`] entity, or the entry root
/// for the entry-level pre-registration) registered into the
/// [`BsxTemplateRegistry`]. Re-registering diffs against it so a name the
/// source no longer defines unregisters; despawning the owner unregisters
/// everything it owned (via the `on_remove` hook), so a deleted source or a
/// torn-down entry scene leaves no stale templates.
#[derive(Debug, Default, Clone, Deref, Component)]
#[component(on_remove = RegisteredTemplates::on_remove())]
struct RegisteredTemplates(Vec<SmolStr>);

/// One template source under a [`TemplateDir`], spawned by its scan: the entity
/// whose [`Blob`] (derived from `rel` against the dir's scoped store) changing
/// re-registers exactly this source, and whose despawn unregisters what it
/// registered.
#[derive(Debug, Clone, Component)]
#[component(on_insert = hook_ext::component_hook(|file: &TemplateFile| BlobPath::derive(&file.rel)))]
pub(crate) struct TemplateFile {
	/// The source's path relative to its dir, naming the module it registers
	/// as (`widgets/Card.bsx` -> `widgets::Card`).
	rel: RelPath,
}

impl TemplateFile {
	/// Read `blob` and re-register it as `file`'s source, parking a pending guard
	/// on the build root (or the file outside a build) so a settle waits on it.
	fn refresh(
		world: &mut World,
		file: Entity,
		rel: RelPath,
		blob: Blob,
		formats: TemplateFormats,
	) {
		let guard = TemplatePending::park_on(
			world,
			TemplateBuildRoot::resolve(world, file),
			PendingKind::Passive,
			format!("`{rel}` re-register"),
		);
		let Ok(mut entity_mut) = world.get_entity_mut(file) else {
			return;
		};
		// local for the same reason the scan is: the bridge poll is only
		// guaranteed on the runtime's local executor.
		entity_mut.run_async_local(async move |file: AsyncEntity| -> Result {
			let source = blob.get().await?.to_vec().xmap(String::from_utf8)?;
			let entity = file.id();
			file.world()
				.with(move |world| -> Result {
					let outcome = TemplateDir::register_sources(
						world,
						entity,
						&formats,
						vec![(rel, source)],
					);
					world.flush();
					guard.resolve(world);
					outcome
				})
				.await
		});
	}
}

/// Re-register each template source whose file changed, through its own
/// [`TemplateFile`]. A file the scan just spawned is skipped: that scan
/// registered its source.
pub(crate) fn register_changed_template_files(
	files: Query<(Entity, &TemplateFile, Ref<Blob>), Changed<Blob>>,
	formats: Res<TemplateFormats>,
	mut commands: Commands,
) {
	for (entity, file, blob) in
		files.iter().filter(|(_, _, blob)| !blob.is_added())
	{
		let (rel, blob, formats) =
			(file.rel.clone(), Blob::clone(&blob), formats.clone());
		commands.queue(move |world: &mut World| {
			TemplateFile::refresh(world, entity, rel, blob, formats)
		});
	}
}

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
	pub fn new(src: impl Into<RelPath>) -> Self { Self { src: src.into() } }

	/// Observer: read the [`TemplateDir`]'s store and register its templates,
	/// one [`TemplateFile`] child per source (see the module docs). A re-insert
	/// (a file created or removed under the dir) re-reads the dir and diffs the
	/// children against it.
	///
	/// The read is store I/O (the filesystem in dev, S3/R2 when deployed), so it
	/// runs as an [`AsyncEntity`] task rather than blocking the runtime (which is
	/// single-threaded on wasm). The dir's scoped [`BlobStore`] (its derived
	/// [`DirPath`]'s output) is resolved *inside* that task, where the whole tree
	/// is built, so it is present; a store-less app is an error. The registration
	/// parks a [`PendingGuard`] on the build root (or this entity outside a
	/// build), so a load or settle ([`TemplatePending::settle`]) waits for it.
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
						.with_state::<Query<&BlobStore>, Result<BlobStore>>(
							move |entity, stores| {
								scoped_store(
									&stores,
									entity,
									"TemplateDir",
									&src,
								)
							},
						)
						.await??;
					let sources = Self::read_sources(&store, &formats).await?;
					dir.world()
						.with(move |world| -> Result {
							Self::sync_files(world, entity, &formats, sources)?;
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

	/// Read every recognized template source under `store` (a template dir's
	/// scoped store) as `(path, source)` pairs (each path relative to it),
	/// keeping only files whose [`MediaType`] `formats` recognizes (`.bsx`,
	/// `.js`). Async (store I/O), so a load path awaits it off the runtime. A
	/// missing directory yields no pairs, so an entry can declare a dir it does
	/// not ship.
	pub async fn read_sources(
		store: &BlobStore,
		formats: &TemplateFormats,
	) -> Result<Vec<(RelPath, String)>> {
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
			.map(async |path| -> Result<(RelPath, String)> {
				let bytes = store.get(&path).await?;
				Ok((path, String::from_utf8(bytes.to_vec())?))
			})
			.xmap(async_ext::try_join_all)
			.await
	}

	/// Register every `(rel, source)` pair `dir` ships, each on its own
	/// [`TemplateFile`] child: a source with a child already is re-registered on
	/// it, a new source gets a child, and a child whose source is gone despawns,
	/// which unregisters its names. One schema refresh for the whole set.
	fn sync_files(
		world: &mut World,
		dir: Entity,
		formats: &TemplateFormats,
		sources: Vec<(RelPath, String)>,
	) -> Result {
		// the files the last scan left, by source path
		let mut existing = world
			.get::<Children>(dir)
			.map(|children| {
				children
					.iter()
					.filter_map(|child| {
						world
							.get::<TemplateFile>(child)
							.map(|file| (file.rel.clone(), child))
					})
					.collect::<HashMap<_, _>>()
			})
			.unwrap_or_default();
		for (rel, source) in sources {
			let file = existing.remove(&rel).unwrap_or_else(|| {
				world
					.spawn((ChildOf(dir), TemplateFile { rel: rel.clone() }))
					.id()
			});
			Self::register_owned(world, file, formats, [(rel, source)])?;
		}
		// the sources the dir no longer ships
		for (_, file) in existing {
			world.entity_mut(file).despawn();
		}
		BsxTemplateRegistry::refresh_schemas(world);
		Ok(())
	}

	/// Register each `(path, source)` pair into the world's [`BsxTemplateRegistry`]
	/// by its module path, lowering each through the format its [`MediaType`]
	/// selects, then refresh the BSX schemas: the entry-level pre-registration,
	/// which owns every entry template on the entry root, and a
	/// [`TemplateFile`]'s re-register.
	///
	/// Registrations are *owned* by `owner`: the owner's previous set is diffed
	/// so a name it no longer defines unregisters, and despawning the owner
	/// unregisters everything it owned (a deleted source, the structural
	/// teardown+rebuild path). Distinct owners still accumulate, so multiple
	/// dirs compose.
	pub fn register_sources(
		world: &mut World,
		owner: Entity,
		formats: &TemplateFormats,
		sources: Vec<(RelPath, String)>,
	) -> Result {
		Self::register_owned(world, owner, formats, sources)?;
		BsxTemplateRegistry::refresh_schemas(world);
		Ok(())
	}

	/// [`register_sources`](Self::register_sources) without the schema refresh,
	/// so a batch of owners refreshes once.
	fn register_owned(
		world: &mut World,
		owner: Entity,
		formats: &TemplateFormats,
		sources: impl IntoIterator<Item = (RelPath, String)>,
	) -> Result {
		let mut registry = world
			.remove_resource::<BsxTemplateRegistry>()
			.unwrap_or_default();
		let names = sources
			.into_iter()
			.map(|(path, source)| {
				registry.insert_source_from_path(formats, &path, &source)
			})
			.collect::<Result<Vec<_>>>();
		// the registry goes back before a bad source's error, never lost with it
		let names = match names {
			Ok(names) => names.into_iter().flatten().collect::<Vec<_>>(),
			Err(err) => {
				world.insert_resource(registry);
				return Err(err);
			}
		};
		// unregister the names this owner previously registered but no longer defines
		if let Some(previous) = world.get::<RegisteredTemplates>(owner) {
			previous
				.iter()
				.filter(|name| !names.contains(name))
				.for_each(|stale| {
					registry.remove(stale);
				});
		}
		world.insert_resource(registry);
		if let Ok(mut owner) = world.get_entity_mut(owner) {
			owner.insert(RegisteredTemplates(names));
		}
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
				.insert(&RelPath::from(*rel), content.to_string())
				.await
				.unwrap();
		}
		store
	}

	/// Inserting a [`TemplateDir`] under a store registers its templates so a
	/// `<widgets::Card>` tag resolves, store-agnostic (wasm too), each source
	/// on its own [`TemplateFile`] child.
	#[beet_core::test]
	async fn registers_templates_from_store() {
		let mut world = router_world();
		let store = memory_fixture(&[(
			"templates/widgets/Card.bsx",
			"<section class=\"card\"><Slot/></section>",
		)])
		.await;
		let root = world
			.spawn((store, children![TemplateDir::new("templates")]))
			.id();
		AsyncRunner::settle_async_tasks(&mut world).await;
		world
			.resource::<BsxTemplateRegistry>()
			.contains("widgets::Card")
			.xpect_true();
		let dir = world.entity(root).get::<Children>().unwrap()[0];
		let files = world.entity(dir).get::<Children>().unwrap();
		files.len().xpect_eq(1);
		world
			.entity(files[0])
			.get::<TemplateFile>()
			.unwrap()
			.rel
			.to_string()
			.xpect_eq("widgets/Card.bsx");
	}

	/// A router world with the main schedule, so the blob reactions run on
	/// [`react`].
	fn reactive_world() -> World {
		(MinimalPlugins, AsyncPlugin, RouterPlugin).into_world()
	}

	/// Run one frame (draining the store's events into the reactions) and settle
	/// the tasks they spawned.
	async fn react(world: &mut World) {
		world.update_local();
		AsyncRunner::settle_async_tasks(world).await;
	}

	/// An in-memory store holding one `Card` template, spawned as its concrete
	/// component (so its watcher subscribes) under a `templates` dir, plus a
	/// handle for writing beside the world. Returns the dir entity too.
	async fn card_site(world: &mut World) -> (BlobStore, Entity) {
		let inner = InMemoryStore::new();
		let handle = BlobStore::new(inner.clone());
		handle
			.insert(
				&RelPath::from("templates/Card.bsx"),
				"<section>first</section>",
			)
			.await
			.unwrap();
		let root = world
			.spawn((inner, children![TemplateDir::new("templates")]))
			.id();
		react(world).await;
		(handle, world.entity(root).get::<Children>().unwrap()[0])
	}

	/// The registered `Card` template's nodes, debug-printed.
	fn card_nodes(world: &World) -> String {
		format!(
			"{:?}",
			world
				.resource::<BsxTemplateRegistry>()
				.get("Card")
				.unwrap()
				.nodes
		)
	}

	/// An edited source re-registers through its own file entity, which keeps
	/// its id: no rescan, no respawn.
	#[beet_core::test]
	async fn edited_source_re_registers_itself() {
		let mut world = reactive_world();
		let (handle, dir) = card_site(&mut world).await;
		card_nodes(&world).xpect_contains("first");
		let file = world.entity(dir).get::<Children>().unwrap()[0];

		handle
			.insert(
				&RelPath::from("templates/Card.bsx"),
				"<section>second</section>",
			)
			.await
			.unwrap();
		react(&mut world).await;
		card_nodes(&world).xpect_contains("second");
		world.entity(dir).get::<Children>().unwrap()[0].xpect_eq(file);
	}

	/// A source created or removed under the dir marks its store changed: the
	/// rescan spawns a file for the new source and despawns the gone one, whose
	/// removal unregisters its names.
	#[beet_core::test]
	async fn created_and_removed_sources_follow_the_dir() {
		let mut world = reactive_world();
		let (handle, dir) = card_site(&mut world).await;

		handle
			.insert(&RelPath::from("templates/Hero.bsx"), "<h1>hero</h1>")
			.await
			.unwrap();
		handle
			.remove(&RelPath::from("templates/Card.bsx"))
			.await
			.unwrap();
		react(&mut world).await;
		let registry = world.resource::<BsxTemplateRegistry>();
		registry.contains("Hero").xpect_true();
		registry.contains("Card").xpect_false();
		world
			.entity(dir)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(1);
	}
}
