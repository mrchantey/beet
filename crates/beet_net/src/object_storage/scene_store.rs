#![allow(missing_docs)]
use crate::prelude::*;
use beet_core::prelude::*;


/// Store and load scenes as needed
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct SceneStore {
	#[reflect(ignore)]
	trigger: CoalescingTrigger,
	/// Load the scene on spawn, defaults to false
	load_on_spawn: bool,
	/// Error when saving an empty scene, defaults to true
	error_on_empty: bool,
}
impl Default for SceneStore {
	fn default() -> Self {
		Self {
			trigger: default(),
			error_on_empty: true,
			load_on_spawn: false,
		}
	}
}

impl SceneStore {
	/// Loads the associated [`Blob`], adding to this entities [`SceneEntities`].
	/// ## Errors
	/// - Errors if this entity has no [`Blob`] or [`SceneStore`]
	pub async fn load(entity: AsyncEntity) -> Result {
		// store is required for writing
		entity.get::<SceneStore, _>(|_| {}).await?;
		let media = entity.get_cloned::<Blob>().await?.get_media().await?;
		entity
			.with_then(move |entity| -> Result {
				SceneLoader::new_entity(entity).load(&media)?;
				Ok(())
			})
			.await?;
		Ok(())
	}
	/// Writes all [`SceneEntities`] and their created descendents to the associated [`Blob`]
	///
	/// ## Errors
	/// - Errors if this entity has no [`Blob`] or [`SceneStore`]
	pub async fn save(entity: AsyncEntity) -> Result {
		let this = entity.get_cloned::<Self>().await?;
		let error_on_empty = this.error_on_empty;
		this.trigger
			.run_flush(async move || {
				let (blob, bytes) = entity
					.with_then(move |mut entity| -> Result<_> {
						let blob = entity.get_or_else::<Blob>()?.clone();

						let spawned_entities =
							entity.try_get::<SceneEntities>()?.to_vec();

						if error_on_empty && spawned_entities.is_empty() {
							bevybail!("cannot save empty scene");
						}

						let world = entity.into_world_mut();
						let mut saver = SceneSaver::new(world);

						for entity in spawned_entities {
							// add all and their descendents to save
							saver = saver.with_entity_tree(entity);
						}
						let scene_media = saver.save(
							blob.media_type().unwrap_or(MediaType::Json),
						)?;
						(blob, scene_media).xok()
					})
					.await?;
				blob.insert(bytes).await?;

				Ok(())
			})
			.await
	}

	/// Temporarily spawn the given bundle to serialize it.
	/// The bundle is spawned with a [`Disabled`]
	pub async fn save_bundle(
		store: AsyncEntity,
		bundle: impl Bundle,
	) -> Result {
		let store_id = store.id();
		let entity = store
			.world()
			.spawn_then((
				bundle,
				SceneOf(store_id),
				// stop CallOnSpawn with Disabled,
				// this will not be serialized
				Disabled,
			))
			.await;
		SceneStore::save(store).await?;
		entity.despawn().await;
		Ok(())
	}
}


pub fn load_scenes_on_insert(
	mut commands: Commands,
	query: Populated<(Entity, &SceneStore), Added<SceneStore>>,
) {
	for (entity, store) in query.iter() {
		if store.load_on_spawn {
			commands.entity(entity).queue_async(SceneStore::load);
		}
	}
}
