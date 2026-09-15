//! [`SceneBlob`]: a scene forked into a store on first boot and booted from the
//! fork thereafter.
//!
//! The `.bsx` is the authored original and no editor path ever writes it; the
//! fork is the living edited representation, a `template_serde` document in
//! the store. One mechanism for every surface: a terminal forks into its store
//! exactly as a browser forks into its local one, and the [`SceneDocument`] the
//! fork lands as is what an inspector edits, the world following per component.
use crate::prelude::*;
use beet_core::prelude::*;

/// The scene at `path` in the nearest ancestor [`BlobStore`], built as this
/// entity's children and landed on it as a [`SceneDocument`].
///
/// On first boot the authored original at `from` (a `.bsx` in the same store)
/// is built, its first entity marked [`SceneFork`], and the built scene saved to
/// `path`; every later boot loads `path` and never reads the original again.
/// Every edit to the scene document is written back to `path`, so an edit
/// outlives the process and the next boot reproduces the edited world.
///
/// ```rsx
/// <SceneBlob path="app.json" from="app.bsx"/>
/// ```
///
/// Lazy in the shape [`DocumentBlob`] settled on: the fork is reached through
/// the [`Blob`] its derived [`BlobPath`] resolves, so a store that has not
/// arrived is not an error, only a boot that has not happened yet.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::component_hook(|blob: &SceneBlob| BlobPath::derive(&blob.path)))]
pub struct SceneBlob {
	/// The fork's path within the nearest ancestor store, the boot source once
	/// it exists. Its extension picks the format, json by default.
	pub path: RelPath,
	/// The authored original's path within the same store, read on first boot
	/// only.
	pub from: RelPath,
	/// Coalesces the write-backs: a burst of edits is at most one write in
	/// flight plus one queued behind it, and the queued one re-reads the
	/// document so it persists the latest state.
	#[reflect(ignore)]
	trigger: CoalescingTrigger,
}

impl SceneBlob {
	/// The scene at `path`, forked from the original at `from`.
	pub fn new(path: impl Into<RelPath>, from: impl Into<RelPath>) -> Self {
		Self {
			path: path.into(),
			from: from.into(),
			trigger: default(),
		}
	}

	/// The fork's format, from its extension.
	fn media_type(&self) -> MediaType {
		self.path.media_type().unwrap_or(MediaType::Json)
	}
}

/// Marks a [`SceneBlob`] whose boot is in flight, so a store churning while it
/// runs does not issue a second one.
#[derive(Component)]
pub(crate) struct ReadingSceneBlob;

/// Boot each unbooted [`SceneBlob`] whose [`Blob`] arrived from its store: the
/// fork when it exists, else the original, forked. Boot-once: a fork edited
/// elsewhere while this world runs is not reloaded over the live scene.
pub(crate) fn read_scene_blobs(
	mut commands: Commands,
	mut async_commands: AsyncCommands,
	blobs: Populated<
		(Entity, &SceneBlob, &Blob),
		(
			Changed<Blob>,
			Without<ReadingSceneBlob>,
			Without<SceneDocument>,
		),
	>,
) {
	for (entity, blob, handle) in blobs.iter() {
		let (blob, store) = (blob.clone(), handle.store().clone());
		commands.entity(entity).insert(ReadingSceneBlob);
		async_commands
			.entity(entity)
			.queue_async(async move |entity| {
				let outcome = read_scene_blob(&entity, blob, store).await;
				// always released, so a transient failure is retried when the next
				// blob or store arrives rather than wedging this one
				entity
					.with(|mut entity| {
						entity.remove::<ReadingSceneBlob>();
					})
					.await?;
				outcome
			});
	}
}

/// The boot itself: load the fork, or build the original and fork it.
async fn read_scene_blob(
	entity: &AsyncEntity,
	blob: SceneBlob,
	store: BlobStore,
) -> Result {
	if store.exists(&blob.path).await? {
		let fork = store.get_media(&blob.path).await?;
		entity
			.with(move |mut entity| -> Result {
				let host = entity.id();
				entity.world_scope(|world| {
					SceneDocument::load_bytes(world, host, &fork)
				})?;
				OK
			})
			.await??;
		return OK;
	}
	let original = store.get_media(&blob.from).await?;
	let (path, from, media_type) =
		(blob.path.clone(), blob.from.clone(), blob.media_type());
	let fork = entity
		.with(move |mut entity| -> Result<MediaBytes> {
			let host = entity.id();
			entity.world_scope(|world| {
				let roots = TemplateLoader::new(world)
					.with_entity(host)
					.load(&original)?;
				// the fork relation, on the scene's first entity so the fork
				// knows its origin wherever the store travels
				world.entity_mut(roots[0]).insert(SceneFork::new(from));
				SceneDocument::fork(world, host, media_type)
			})
		})
		.await??;
	store.insert(&path, fork.take().1).await
}

/// Write each edited scene document back to its store.
pub(crate) fn write_scene_blobs(
	mut async_commands: AsyncCommands,
	edited: Populated<
		(Entity, &SceneBlob, &Blob),
		(With<SceneDocument>, Changed<Document>),
	>,
	just_loaded: Query<(), Added<SceneDocument>>,
) {
	for (entity, blob, handle) in edited.iter() {
		// the frame the boot landed is not an edit: writing it back would echo
		// the file at itself
		if just_loaded.contains(entity) {
			continue;
		}
		let (handle, media_type, trigger) =
			(handle.clone(), blob.media_type(), blob.trigger.clone());
		async_commands
			.entity(entity)
			.queue_async(async move |entity| {
				trigger
					.run_flush(async || {
						write_scene_blob(&entity, &handle, media_type.clone())
							.await
					})
					.await
			});
	}
}

/// One write: the document as it stands *now*, through the format's own
/// serializer so entity order and the sorted component maps match the fork.
async fn write_scene_blob(
	entity: &AsyncEntity,
	blob: &Blob,
	media_type: MediaType,
) -> Result {
	let subject = blob.path().clone();
	let bytes = entity
		.with(move |entity| -> Result<MediaBytes> {
			let document = entity.get::<Document>().ok_or_else(|| {
				bevyhow!("`{subject}` holds no scene document to write")
			})?;
			let registry = entity.world().resource::<AppTypeRegistry>().read();
			SceneDocument::to_bytes(&registry, &document.0, media_type)
		})
		.await??;
	blob.insert(bytes.take().1).await
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The type path a text node's content is stored under.
	const VALUE: &str = "beet_core::types::value::value::Value";

	/// A store holding the authored original: a heading over two items.
	async fn store() -> BlobStore {
		let store = BlobStore::temp();
		store
			.insert(
				&RelPath::from("app.bsx"),
				"<main><h1>Todos</h1><span>milk</span></main>",
			)
			.await
			.unwrap();
		store
	}

	/// Boot an app whose entry is `<SceneBlob>` under `store`, settled.
	async fn boot(store: BlobStore) -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AsyncPlugin,
			TemplatePlugin,
			DocumentPlugin,
			StorePlugin,
		));
		app.init_plugin::<MinimalTypesPlugin>();
		let host = app
			.world_mut()
			.spawn((store, children![SceneBlob::new("app.json", "app.bsx")]))
			.flush();
		let host = app.world().entity(host).get::<Children>().unwrap()[0];
		app.update_async().await;
		app.update_async().await;
		(app, host)
	}

	/// First boot builds the original, forks it into the store with the fork
	/// relation recorded, and lands the scene document on the host.
	#[beet_core::test]
	async fn a_first_boot_forks_the_original() {
		let store = store().await;
		let (app, host) = boot(store.clone()).await;
		store
			.exists(&RelPath::from("app.json"))
			.await
			.unwrap()
			.xpect_true();
		let world = app.world();
		world.get::<SceneDocument>(host).xpect_some();
		let root = world
			.get::<TemplateEntityMap>(host)
			.unwrap()
			.world(0)
			.unwrap();
		world
			.get::<SceneFork>(root)
			.unwrap()
			.from
			.as_str()
			.xpect_eq("app.bsx");
		world.get::<ChildOf>(root).unwrap().parent().xpect_eq(host);
		// the fork carries the record, and reads back as the document
		let fork = store.get_media(&RelPath::from("app.json")).await.unwrap();
		let json: serde_json::Value =
			serde_json::from_slice(fork.bytes()).unwrap();
		SceneEntities::entity_json(&json, 0)
			.get("beet_core::template_serde::scene_document::SceneFork")
			.xpect_some();
	}

	/// The published page: a first boot through a store fork finds the scene
	/// fork upstream and reads it, writing nothing; the first edit lands in the
	/// local half, and a reboot reads the local copy over the published one.
	#[beet_core::test]
	async fn a_first_boot_through_a_store_fork_reads_upstream() {
		// the server's own boot publishes the fork upstream
		let upstream = store().await;
		let (app, host) = boot(upstream.clone()).await;
		let key = heading_key(&app, host);
		drop(app);
		let local = BlobStore::temp();
		let fork =
			BlobStore::new(StoreFork::new(local.clone(), upstream.clone()));
		let (mut app, host) = boot(fork.clone()).await;
		app.world().get::<SceneDocument>(host).xpect_some();
		local
			.exists(&RelPath::from("app.json"))
			.await
			.unwrap()
			.xpect_false();
		// the first edit is the visitor's fork
		edit_heading(&mut app, host, key, "Groceries").await;
		local
			.exists(&RelPath::from("app.json"))
			.await
			.unwrap()
			.xpect_true();
		upstream
			.get_media(&RelPath::from("app.json"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xnot()
			.xpect_contains("Groceries");
		let (rebooted, host) = boot(fork).await;
		heading(&rebooted, host, key).xpect_eq("Groceries");
	}

	/// The file key of the entity whose text is the heading.
	fn heading_key(app: &App, host: Entity) -> u32 {
		let scene = &app.world().get::<Document>(host).unwrap().0;
		let entities = SceneEntities::of(scene).unwrap();
		entities
			.keys()
			.unwrap()
			.into_iter()
			.find(|key| {
				entities
					.component(*key, VALUE)
					.is_some_and(|value| value.as_str().ok() == Some("Todos"))
			})
			.unwrap()
	}

	/// Retype the heading through the document, settled so the world followed
	/// and the write landed.
	async fn edit_heading(app: &mut App, host: Entity, key: u32, text: &str) {
		SceneEntities::of_mut(
			&mut app.world_mut().get_mut::<Document>(host).unwrap().0,
		)
		.unwrap()
		.insert_component(key, VALUE, text)
		.unwrap();
		app.update_async().await;
		app.update_async().await;
	}

	/// The heading's live text.
	fn heading(app: &App, host: Entity, key: u32) -> String {
		let entity = app
			.world()
			.get::<TemplateEntityMap>(host)
			.unwrap()
			.world(key)
			.unwrap();
		app.world()
			.get::<Value>(entity)
			.unwrap()
			.as_str()
			.unwrap()
			.to_string()
	}

	/// An edit reaches the store, and a reboot from the store reproduces the
	/// edited world rather than the original.
	#[beet_core::test]
	async fn an_edit_persists_across_a_reboot() {
		let store = store().await;
		let (mut app, host) = boot(store.clone()).await;
		// the element entities carry `Element`, and a text node its `Value`;
		// rename the heading by editing the text
		let key = heading_key(&app, host);
		edit_heading(&mut app, host, key, "Groceries").await;
		// the live world followed
		heading(&app, host, key).xpect_eq("Groceries");

		// a fresh process over the same store boots from the fork
		let (rebooted, host) = boot(store).await;
		heading(&rebooted, host, key).xpect_eq("Groceries");
	}
}
