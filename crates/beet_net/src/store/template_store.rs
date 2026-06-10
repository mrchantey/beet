#![allow(missing_docs)]
use crate::prelude::*;
use beet_core::prelude::*;


/// Store and load serialized world data as needed
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct TemplateStore {
	#[reflect(ignore)]
	trigger: CoalescingTrigger,
	/// Load the serialized data on spawn, defaults to false
	load_on_spawn: bool,
	/// Error when saving empty data, defaults to true
	error_on_empty: bool,
}
impl Default for TemplateStore {
	fn default() -> Self {
		Self {
			trigger: default(),
			error_on_empty: true,
			load_on_spawn: false,
		}
	}
}

impl TemplateStore {
	pub async fn load_or_create<
		T: 'static + Send + Sync + Clone + Reflect + BlobStoreProvider,
		B: Bundle,
	>(
		world: AsyncWorld,
		blob: TypedBlob<T>,
		create: impl 'static + Send + Sync + AsyncFnOnce(AsyncEntity) -> Result<B>,
	) -> Result<Vec<Entity>> {
		let entity = world.spawn((blob, Self::default())).await;
		Self::load_or_create_inner(entity, create).await
	}

	async fn load_or_create_inner<B: Bundle>(
		entity: AsyncEntity,
		create: impl 'static + Send + Sync + AsyncFnOnce(AsyncEntity) -> Result<B>,
	) -> Result<Vec<Entity>> {
		if !entity.get_cloned::<Blob>().await?.exists().await? {
			let bundle = create(entity.clone()).await?;
			Self::save_bundle(entity.clone(), bundle).await?;
		}
		Self::load(entity).await
	}


	/// Loads the associated [`Blob`], adding to this entities [`TemplateNodes`].
	/// ## Errors
	/// - Errors if this entity has no [`Blob`] or [`TemplateStore`]
	pub async fn load(entity: AsyncEntity) -> Result<Vec<Entity>> {
		// store is required for writing
		entity.get::<TemplateStore, _>(|_| {}).await?;
		let media = entity.get_cloned::<Blob>().await?.get_media().await?;
		entity
			.with(move |entity| -> Result<_> {
				TemplateLoader::new_entity(entity).load(&media)
			})
			.await
			.flatten()
	}
	/// Writes all [`TemplateNodes`] and their created descendents to the associated [`Blob`]
	///
	/// ## Errors
	/// - Errors if this entity has no [`Blob`] or [`TemplateStore`]
	pub async fn save(entity: AsyncEntity) -> Result {
		let this = entity.get_cloned::<Self>().await?;
		let error_on_empty = this.error_on_empty;
		this.trigger
			.run_flush(async move || {
				let (blob, bytes) = entity
					.with(move |mut entity| -> Result<_> {
						let blob = entity.get_or_else_mut::<Blob>()?.clone();

						let spawned_entities =
							entity.try_get::<TemplateNodes>()?.to_vec();

						if error_on_empty && spawned_entities.is_empty() {
							bevybail!("cannot save empty world serde data");
						}

						let world = entity.into_world_mut();
						let mut saver = TemplateSaver::new();

						for entity in spawned_entities {
							// add all and their descendents to save
							saver = saver.with_entity_tree(world, entity);
						}
						let template_media = saver.save(
							world,
							blob.media_type().unwrap_or(MediaType::Json),
						)?;
						(blob, template_media).xok()
					})
					.await??;
				blob.insert(bytes).await?;

				Ok(())
			})
			.await
	}

	/// Temporarily spawn the given bundle to serialize it
	/// to the blob associated with this entity.
	/// The spawn/despawn step is exclusive and synchronous,
	/// ensuring no systems run.
	/// ## Errors
	/// Errors if the entity has no blob
	pub async fn save_bundle(
		store: AsyncEntity,
		bundle: impl Bundle,
	) -> Result {
		let blob = store.get_cloned::<Blob>().await?;
		let media_type = blob.media_type().unwrap_or(MediaType::Json);
		let bytes = store
			.world()
			.with(move |world| {
				let entity = world.spawn(bundle).id();
				let bytes = TemplateSaver::new()
					.with_entity_tree(world, entity)
					.save(world, media_type);
				world.entity_mut(entity).despawn();
				bytes
			})
			.await?;
		blob.insert(bytes).await?;
		Ok(())
	}
}


pub fn load_template_on_insert(
	async_commands: AsyncCommands,
	query: Populated<(Entity, &TemplateStore), Added<TemplateStore>>,
) {
	for (entity, store) in query.iter() {
		if store.load_on_spawn {
			async_commands.entity(entity).run(async |entity| {
				TemplateStore::load(entity).await?;
				Ok(())
			});
		}
	}
}
