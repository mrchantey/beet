use beet_core::prelude::*;

/// Maps a clip asset path to its node index in the host entity's
/// [`AnimationGraph`], recorded when the graph is built so an action resolves its
/// clip path to a node once, replacing the per-frame `Added<AnimationPlayer>`
/// search in the old `resolve_animation_clips`.
#[derive(Debug, Default, Component, Deref)]
pub struct AnimationGraphClips(HashMap<SmolStr, AnimationNodeIndex>);

impl AnimationGraphClips {
	/// Create from a clip-path -> node-index map.
	pub fn new(map: HashMap<SmolStr, AnimationNodeIndex>) -> Self { Self(map) }
	/// The graph node index registered for the clip at `path`, if any.
	pub fn index(&self, path: &str) -> Option<AnimationNodeIndex> {
		self.0.get(path).copied()
	}
}

/// Build an [`AnimationGraph`] from `clips` (asset paths), loading each clip
/// through the deferred path ([`BuildAssets`]) so `LoadTemplate` waits for every
/// clip, and returning the graph handle and the clip-path -> node-index map
/// ([`AnimationGraphClips`]) for actions to resolve against.
///
/// [`AnimationTransitions`] is *not* part of this bundle: it belongs on the glb's
/// [`AnimationPlayer`] (a descendant spawned later), where `init_scene_animators`
/// adds it. Putting it on this (player-less) agent root would be dead weight bevy
/// never advances, and a split-brain hazard against the real player's.
///
/// The reusable core of a markup `<CreateAnimationGraph>` template, replacing the
/// imperative `AnimationGraph::new()` + `graphs.add(..)` scene templates built by
/// hand. A `RecursiveDependencyLoadState` on the graph handle does *not* cover its
/// clips, so each clip handle is parked as its own pending dependency.
pub fn build_animation_graph(
	clips: &[String],
	graphs: &mut Assets<AnimationGraph>,
	assets: &mut BuildAssets,
) -> impl Bundle {
	let mut graph = AnimationGraph::new();
	let root = graph.root;
	let indices = clips
		.iter()
		.map(|path| {
			let handle = assets.load::<AnimationClip>(path.clone());
			(SmolStr::new(path), graph.add_clip(handle, 1.0, root))
		})
		.collect();
	(
		AnimationGraphHandle(graphs.add(graph)),
		AnimationGraphClips(indices),
	)
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::asset::AssetApp;
	use bevy::asset::AssetPlugin;

	#[beet_core::test]
	fn defers_clips_and_maps_indices() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default(), TemplatePlugin))
			.init_asset::<AnimationClip>()
			.init_asset::<AnimationGraph>();
		let world = app.world_mut();

		let fired = Store::new(false);
		let f = fired.clone();
		world.add_observer(move |_: On<LoadTemplate>| f.set(true));

		let clips =
			vec!["fox.glb#Animation0".to_string(), "fox.glb#Animation1".to_string()];
		let root = world
			.spawn_template(system_template::<
				(ResMut<Assets<AnimationGraph>>, BuildAssets),
				_,
				_,
			>(move |_entity, (mut graphs, mut assets)| {
				Snippet::from_bundle(build_animation_graph(
					&clips,
					&mut graphs,
					&mut assets,
				))
			}))
			.unwrap()
			.id();

		// LoadTemplate deferred: both clips parked pending on the build root.
		fired.get().xpect_false();
		world.entity(root).contains::<PendingAssets>().xpect_true();
		// the clip-path -> node-index map is recoverable, distinct per clip.
		let clips = world.entity(root).get::<AnimationGraphClips>().unwrap();
		let idle = clips.index("fox.glb#Animation0").unwrap();
		let walk = clips.index("fox.glb#Animation1").unwrap();
		(idle == walk).xpect_false();
	}
}
