use beet_core::prelude::*;

pub struct Foxie {
	pub idle_index: AnimationNodeIndex,
	pub idle_clip: Handle<AnimationClip>,
	pub walk_index: AnimationNodeIndex,
	pub walk_clip: Handle<AnimationClip>,
	pub graph_handle: AnimationGraphHandle,
}

impl Foxie {
	pub fn new(
		asset_server: &AssetServer,
		graphs: &mut ResMut<Assets<AnimationGraph>>,
	) -> Foxie {
		let mut graph = AnimationGraph::new();
		let idle_clip =
			asset_server.load::<AnimationClip>("misc/fox.glb#Animation0");
		let idle_index = graph.add_clip(idle_clip.clone(), 1.0, graph.root);
		let walk_clip =
			asset_server.load::<AnimationClip>("misc/fox.glb#Animation1");
		let walk_index = graph.add_clip(walk_clip.clone(), 1.0, graph.root);

		let graph_handle = AnimationGraphHandle(graphs.add(graph));

		Foxie {
			graph_handle,
			idle_index,
			idle_clip,
			walk_index,
			walk_clip,
		}
	}
}
