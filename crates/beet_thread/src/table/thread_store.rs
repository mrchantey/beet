use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


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
					SceneLoader::new(world).load_json(&scene_bytes)?;
					Ok(())
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

/// Write the thread scene to its blob, coalescing concurrent requests.
///
/// Only one write runs in-flight at a time. If called while a write is already
/// in-flight, the dirty flag is set and the in-flight write will re-run once
/// after it finishes — regardless of how many extra calls arrive.
pub async fn write(entity: AsyncEntity, trigger: CoalescingTrigger) -> Result {
	// If a write is already in-flight, the dirty flag is set for a retry
	if !trigger.start() {
		return Ok(());
	}
	// Drive until no pending dirty requests remain
	loop {
		write_inner(entity).await?;
		if !trigger.finish() {
			break;
		}
	}
	Ok(())
}

async fn write_inner(entity: AsyncEntity) -> Result {
	let (blob, json) = entity
		.with_then(|mut entity| -> Result<_> {
			let thread_root = entity
				.with_state::<ThreadQuery, Result<Entity>>(
					|entity, query| query.thread(entity)?.entity.xok(),
				)?;

			let root_ancestor =
				entity.with_state::<AncestorQuery, _>(|entity, query| {
					query.root_ancestor(entity)
				});

			let world = entity.into_world_mut();
			let blob = world.entity(thread_root).get_or_else::<Blob>()?.clone();
			let json = SceneSaver::new(world)
				.with_entity_tree(root_ancestor)
				.save_json()?;
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
	mut triggers: Local<HashMap<Entity, CoalescingTrigger>>,
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
		// get or create a per-entity coalescing trigger
		let trigger = triggers.entry(thread.entity).or_default().clone();
		commands
			.entity(thread.entity)
			.queue_async(move |entity| async move {
				thread_store::write(entity, trigger).await
			});
	}

	Ok(())
}
