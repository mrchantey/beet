use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


pub fn store_thread_on_post(
	mut commands: Commands,
	changed: Query<(Entity, &Post), Changed<Post>>,
	stores: AncestorQuery<&SpawnedBy>,
) -> Result {
	for (entity, post) in changed.iter() {
		if post.in_progress() {
			// do not react to in-progress posts, ie streaming text.
			continue;
		}
		let Ok(store) = stores.get(entity) else {
			// this post is not in a scene spawned hierarchy
			continue;
		};
		commands
			.entity(**store)
			.queue_async(move |entity| async move {
				SceneStore::save(entity).await
			});
	}

	Ok(())
}
