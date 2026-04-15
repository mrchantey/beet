#![allow(missing_docs)]
use crate::prelude::*;
use beet_core::prelude::*;


/// Store and load scenes as needed
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct SceneStore {
	#[reflect(ignore)]
	trigger: CoalescingTrigger,
	/// Load the scene on spawn, defaults to true
	load_on_spawn: bool,
	/// Error when saving an emtpy scene, defaults to true
	error_on_empty: bool,
}
impl Default for SceneStore {
	fn default() -> Self {
		Self {
			trigger: default(),
			error_on_empty: true,
			load_on_spawn: true,
		}
	}
}

impl SceneStore {
	/// Loads the associated [`Blob`], adding to this entities [`SpawnedEntities`].
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
	/// Writes all [`SpawnedEntities`] to the associated [`Blob`]
	///
	/// ## Errors
	/// - Errors if this entity has no [`Blob`] or [`SceneStore`]
	pub async fn save(entity: AsyncEntity) -> Result {
		let this = entity.get_cloned::<Self>().await?;
		let error_on_empty = this.error_on_empty;
		this.trigger
			.run_flush(async move || {
				let (blob, bytes) = entity
					.with_then(|mut entity| -> Result<_> {
						let blob = entity.get_or_else::<Blob>()?.clone();

						// errors if empty, nothing to save.
						let spawned_entities = entity
							.get_or_else::<SpawnedEntities>()?
							// TODO error_on_empty weird lifetime issue
							.to_vec();

						let world = entity.into_world_mut();
						let mut saver = SceneSaver::new(world);

						for entity in spawned_entities {
							// add all to save
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
