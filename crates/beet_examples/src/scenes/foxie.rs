use crate::prelude::*;
use bevy::prelude::*;

pub struct Foxie {
	pub graph: AnimationGraphPlaceholder,
	pub idle_clip: AssetPlaceholder<AnimationClip>,
	pub idle_index: AnimationNodeIndex,
	pub walk_clip: AssetPlaceholder<AnimationClip>,
	pub walk_index: AnimationNodeIndex,
}


pub fn load_foxie() -> Foxie {
	let mut graph = AnimationGraphPlaceholder::default();

	let idle_clip = AssetPlaceholder::<AnimationClip>::new("Fox.glb#Animation0");
	let idle_index = graph.add_clip(idle_clip.clone(), 1.0, graph.root);
	let walk_clip = AssetPlaceholder::<AnimationClip>::new("Fox.glb#Animation1");
	let walk_index = graph.add_clip(walk_clip.clone(), 1.0, graph.root);

	Foxie {
		graph,
		idle_clip,
		idle_index,
		walk_clip,
		walk_index,
	}
}
