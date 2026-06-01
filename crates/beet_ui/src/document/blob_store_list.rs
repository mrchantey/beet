use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Reactive list of the objects in an entity's [`BlobStore`].
///
/// Spawns one [`ReactiveChild`] per object, keyed on `Changed<BlobStore>`: when
/// a typed store's watcher fires, propagation marks the erased [`BlobStore`] on
/// the same entity `Changed` and [`refresh_blob_store_list`] re-lists it into the
/// backing field, which the [`ReactiveChildren`] chain renders.
///
/// The concrete provider component is supplied beside it by the caller (its
/// `on_add` inserts the [`BlobStore`] and triggers the watcher):
///
/// ```no_run
/// # use beet_ui::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_core::prelude::*;
/// # let mut world = World::new();
/// world.spawn((
///     Document::default(),
///     InMemoryStore::new(),
///     BlobStoreList::new(|_, path| OnSpawn::insert(path.clone())),
/// ));
/// ```
pub struct BlobStoreList;

impl BlobStoreList {
	/// One reactive row per object in the entity's [`BlobStore`], rebuilt as it
	/// changes.
	pub fn new(
		build_item: impl 'static + Send + Sync + Fn(usize, &Value) -> OnSpawn,
	) -> impl Bundle {
		let field = TypedFieldRef::<Vec<SmolPath>>::inline();
		ReactiveChildren::new(field.field(), build_item)
	}
}

/// Re-lists each [`BlobStore`] into its backing field when it changes.
///
/// Component insertion counts as `Changed`, so the initial list populates
/// without special-casing. Because `list` is async, the external-change-to-rows
/// path is multi-frame: this frame spawns the list task, a later frame writes
/// the field at the sync point and converges.
pub(super) fn refresh_blob_store_list(
	stores: Populated<
		(Entity, &BlobStore, &FieldRef),
		Changed<BlobStore>,
	>,
	commands: AsyncCommands,
) {
	for (entity, store, field) in stores.iter() {
		let store = store.clone();
		let field = TypedFieldRef::<Vec<SmolPath>>::from_field(field.clone());
		commands.entity(entity).run_local(async move |entity| {
			// graceful empty, like ListBlobs
			let paths = store.list().await.unwrap_or_default();
			entity
				.with_state::<FieldQuery, _>(move |subject, mut fields| {
					fields.set_typed(subject, &field, paths)
				})
				.await?
		});
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	/// Count the `ReactiveChild` rows of `entity`.
	fn row_count(world: &mut World, entity: Entity) -> usize {
		world
			.entity(entity)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.filter(|child| {
						world.entity(*child).contains::<ReactiveChild>()
					})
					.count()
			})
			.unwrap_or(0)
	}

	#[beet_core::test]
	async fn lists_and_reacts() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin, DocumentPlugin));
		// the InMemoryStore is co-located with the Document + list; `store`
		// shares its backing Arc so test writes reach the same objects
		let inner = InMemoryStore::new();
		let store = BlobStore::new(inner.clone());
		let entity = app
			.world_mut()
			.spawn((
				Document::default(),
				inner,
				BlobStoreList::new(|_, value| OnSpawn::insert(value.clone())),
			))
			.id();

		// seed two objects, drive async until the refresh converges
		store.insert(&SmolPath::new("a.txt"), "a").await.unwrap();
		store.insert(&SmolPath::new("b.txt"), "b").await.unwrap();
		app.update_async().await;
		row_count(app.world_mut(), entity).xpect_eq(2);

		// remove one, the Removed event marks the store Changed and re-lists
		store.remove(&SmolPath::new("a.txt")).await.unwrap();
		app.update_async().await;
		row_count(app.world_mut(), entity).xpect_eq(1);
	}
}
