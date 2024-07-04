use super::*;
use bevy::prelude::*;

pub struct AnimationGraphPlaceholderPlugin;
impl Plugin for AnimationGraphPlaceholderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, init_animation_asset)
			.register_type::<AnimationGraphPlaceholder>();
	}
}


#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct AnimationGraphPlaceholder {
	pub root: AnimationNodeIndex,
	pub clips: Vec<AnimationClipPlaceholder>,
}

impl AnimationGraphPlaceholder {
	pub fn add_clip(
		&mut self,
		clip: AssetPlaceholder<AnimationClip>,
		weight: f32,
		parent: AnimationNodeIndex,
	) -> AnimationNodeIndex {
		self.clips.push(AnimationClipPlaceholder {
			clip,
			parent,
			weight,
		});
		// thie first index is root
		// we can safely determine the indices because we control
		// the order in which they are added, see [`init_animation_asset`]
		AnimationNodeIndex::new(self.clips.len())
	}
}


#[derive(Debug, Reflect)]
pub struct AnimationClipPlaceholder {
	pub clip: AssetPlaceholder<AnimationClip>,
	pub parent: AnimationNodeIndex,
	pub weight: f32,
}

/// This works in unison with the [`AssetPlaceholderPlugin`]
/// [`init_asset`]
/// [`init_asset`]: super::init_asset
pub fn init_animation_asset(
	mut commands: Commands,
	mut lookup: ResMut<AssetPlaceholderLookup<AnimationClip>>,
	mut asset_server: ResMut<AssetServer>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
	query: Query<
		(Entity, &AnimationGraphPlaceholder),
		Added<AnimationGraphPlaceholder>,
	>,
) {
	for (entity, placeholder) in query.iter() {
		let mut graph = AnimationGraph::new();

		for clip in &placeholder.clips {
			let handle =
				lookup.get_or_create(&mut asset_server, &clip.clip.path);
			graph.add_clip(handle, clip.weight, clip.parent);
		}
		let handle = graphs.add(graph);

		commands
			.entity(entity)
			.insert(handle)
			.remove::<AnimationGraphPlaceholder>();
	}
}
