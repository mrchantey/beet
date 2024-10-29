use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::scene::SceneInstanceReady;


/// Currently [`RenderLayers`] are not applied to children of a scene.
/// This [`SceneInstanceReady`] observer applies the [`RenderLayers`]
/// of a [`SceneRoot`] to all children with a [`Transform`].
pub fn apply_render_layers_to_children(
	trigger: Trigger<SceneInstanceReady>,
	mut commands: Commands,
	children: Query<&Children>,
	transforms: Query<&Transform>,
	query: Populated<(Entity, &RenderLayers)>,
) {
	let Ok((parent, render_layers)) = query.get(trigger.entity()) else {
		return;
	};
	children.iter_descendants(parent).for_each(|entity| {
		if transforms.contains(entity) {
			commands.entity(entity).insert(render_layers.clone());
		}
	});
}
