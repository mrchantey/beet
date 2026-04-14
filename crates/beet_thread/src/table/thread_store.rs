use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Default, Clone)]
struct CoalescingTrigger(Arc<Mutex<CoalescingTriggerInner>>);
#[derive(Debug, Default, Clone)]
struct CoalescingTriggerInner {
	dirty: bool,
	in_progress: bool,
}

pub async fn load_or_spawn<Out>(
	world: AsyncWorld,
	blob: Blob,
	scene: impl 'static + Send + Sync + FnOnce(&mut World) -> Out,
) -> Result
where
	Out: IntoResult,
{
	match blob.get().await {
		Ok(scene_bytes) => {
			// Scene exists, load it and call the root
			world
				.with_then(move |world| -> Result {
					SceneLoader::new(world).load_json(&scene_bytes)
				})
				.await?;
		}
		Err(_) => {
			// No existing scene, create fresh
			world.with_then(|world| scene(world).into_result()).await?;
		}
	}
	Ok(())
}

pub async fn write(entity: AsyncEntity) -> Result {
	let (blob, json) = entity
		.with_then(|mut entity| -> Result<_> {
			let root = entity.with_state::<ThreadQuery, Result<Entity>>(
				|entity, query| query.thread(entity)?.entity.xok(),
			)?;
			let world = entity.into_world_mut();
			let blob = world.entity(root).get_or_else::<Blob>()?.clone();
			let json =
				SceneSaver::new(world).with_entity_tree(root).save_json()?;
			(blob, json).xok()
		})
		.await?;
	blob.insert(json).await?;
	Ok(())
}



pub fn store_thread_on_post(
	mut commands: Commands,
	threads: ThreadQuery,
	changed: Query<(Entity, &Post), Changed<Post>>,
) -> Result {
	for (entity, post) in changed.iter() {
		if post.in_progress() {
			// do not react to in-progress posts, ie streaming text.
			continue;
		}
		let thread = threads.thread(entity)?;
		if thread.blob.is_none() {
			// no blob, nothing to save to
			continue;
		}
		commands
			.entity(thread.entity)
			.queue_async(async |entity| thread_store::write(entity).await);
	}

	Ok(())
}
