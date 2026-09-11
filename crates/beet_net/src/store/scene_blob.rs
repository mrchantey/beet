//! [`SceneBlob`]: a scene forked into a store on first boot and booted from the
//! fork thereafter.
//!
//! The `.bsx` is the authored original and no editor path ever writes it; the
//! fork is the living edited representation, a `template_serde` document in
//! the store. One mechanism for every surface: a terminal forks into its store
//! exactly as a browser forks into its local one, and the [`SceneDocument`] the
//! fork lands as is what an inspector edits, the world following per component.
use super::ancestor_store;
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
/// Lazy in the shape [`DocumentBlob`] settled on: a store that has not arrived
/// is not an error, only a boot that has not happened yet.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct SceneBlob {
	/// The fork's path within the nearest ancestor store, the boot source once
	/// it exists. Its extension picks the format, json by default.
	pub path: SmolPath,
	/// The authored original's path within the same store, read on first boot
	/// only.
	pub from: SmolPath,
	/// Coalesces the write-backs: a burst of edits is at most one write in
	/// flight plus one queued behind it, and the queued one re-reads the
	/// document so it persists the latest state.
	#[reflect(ignore)]
	trigger: CoalescingTrigger,
}

impl SceneBlob {
	/// The scene at `path`, forked from the original at `from`.
	pub fn new(path: impl Into<SmolPath>, from: impl Into<SmolPath>) -> Self {
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

/// Run condition for [`read_scene_blobs`]: a blob or a store arrived, the two
/// orders in which a stored scene becomes bootable.
pub(crate) fn scene_blobs_may_be_readable(
	blobs: Query<(), Added<SceneBlob>>,
	stores: Query<(), Added<BlobStore>>,
) -> bool {
	!blobs.is_empty() || !stores.is_empty()
}

/// Boot each [`SceneBlob`] from its nearest ancestor store: the fork when it
/// exists, else the original, forked.
pub(crate) fn read_scene_blobs(
	mut commands: Commands,
	async_commands: AsyncCommands,
	blobs: Populated<
		(Entity, &SceneBlob),
		(Without<ReadingSceneBlob>, Without<SceneDocument>),
	>,
) {
	for (entity, blob) in blobs.iter() {
		let blob = blob.clone();
		commands.entity(entity).insert(ReadingSceneBlob);
		async_commands.entity(entity).run(async move |entity| {
			let outcome = read_scene_blob(&entity, blob).await;
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
async fn read_scene_blob(entity: &AsyncEntity, blob: SceneBlob) -> Result {
	let Some(store) = ancestor_store(entity).await? else {
		return OK;
	};
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
	async_commands: AsyncCommands,
	edited: Populated<
		(Entity, &SceneBlob),
		(With<SceneDocument>, Changed<Document>),
	>,
	just_loaded: Query<(), Added<SceneDocument>>,
) {
	for (entity, blob) in edited.iter() {
		// the frame the boot landed is not an edit: writing it back would echo
		// the file at itself
		if just_loaded.contains(entity) {
			continue;
		}
		let (path, media_type, trigger) =
			(blob.path.clone(), blob.media_type(), blob.trigger.clone());
		async_commands.entity(entity).run(async move |entity| {
			trigger
				.run_flush(async move || {
					write_scene_blob(&entity, path.clone(), media_type.clone())
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
	path: SmolPath,
	media_type: MediaType,
) -> Result {
	let Some(store) = ancestor_store(entity).await? else {
		return OK;
	};
	let subject = path.clone();
	let bytes = entity
		.with(move |entity| -> Result<MediaBytes> {
			let document = entity.get::<Document>().ok_or_else(|| {
				bevyhow!("`{subject}` holds no scene document to write")
			})?;
			let registry = entity.world().resource::<AppTypeRegistry>().read();
			SceneDocument::to_bytes(&registry, &document.0, media_type)
		})
		.await??;
	store.insert(&path, bytes.take().1).await
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
				&SmolPath::from("app.bsx"),
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
			.exists(&SmolPath::from("app.json"))
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
		let fork = store.get_media(&SmolPath::from("app.json")).await.unwrap();
		let json: serde_json::Value =
			serde_json::from_slice(fork.bytes()).unwrap();
		json["entities"]["0"]["components"]
			.get("beet_core::template_serde::scene_document::SceneFork")
			.xpect_some();
	}

	/// An edit reaches the store, and a reboot from the store reproduces the
	/// edited world rather than the original.
	#[beet_core::test]
	async fn an_edit_persists_across_a_reboot() {
		let store = store().await;
		let (mut app, host) = boot(store.clone()).await;
		// the element entities carry `Element`, and a text node its `Value`;
		// rename the heading by editing the text
		let key = {
			let scene = &app.world().get::<Document>(host).unwrap().0;
			let entities = SceneEntities::of(scene).unwrap();
			entities
				.keys()
				.unwrap()
				.into_iter()
				.find(|key| {
					entities
						.components(*key)
						.and_then(|components| components.get(VALUE).ok())
						.is_some_and(|value| {
							value.as_str().ok() == Some("Todos")
						})
				})
				.unwrap()
		};
		app.world_mut()
			.get_mut::<Document>(host)
			.unwrap()
			.0
			.get_mut("entities")
			.unwrap()
			.get_mut(&key.to_string())
			.unwrap()
			.get_mut("components")
			.unwrap()
			.insert(VALUE, "Groceries")
			.unwrap();
		app.update_async().await;
		app.update_async().await;
		// the live world followed
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
			.xpect_eq("Groceries");

		// a fresh process over the same store boots from the fork
		let (rebooted, host) = boot(store).await;
		let entity = rebooted
			.world()
			.get::<TemplateEntityMap>(host)
			.unwrap()
			.world(key)
			.unwrap();
		rebooted
			.world()
			.get::<Value>(entity)
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("Groceries");
	}
}
