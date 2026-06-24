use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// The capabilities every beet example shares: the action + spatial systems, the
/// 2d/3d helper systems, the terminal UI, and the window conveniences (esc to
/// close, F11 fullscreen). The runner and window are [`BeetPlugins`](beet)' job;
/// this only adds behaviour, so it composes under either runner.
pub fn beet_example_plugin(app: &mut App) {
	assert_local_assets();

	// `ActionPlugin` may already be present (the router stack adds it via
	// `init_plugin` on the CLI render path), so add it idempotently rather than
	// panic on the double-add when this group composes with a router-bearing app.
	app.init_plugin::<ActionPlugin>()
		.add_plugins((
			BeetSpatialPlugins,
			plugin_2d,
			plugin_3d,
			UiTerminalPlugin,
		))
		.add_systems(
			Update,
			(
				close_on_esc,
				toggle_fullscreen,
				ensure_spatial_roots,
				wire_behaviour_agents,
			),
		)
		.init_resource::<RandomSource>()
		.register_type::<Collectable>();
}

/// In a `.bsx` a behaviour tree is nested under its agent (eg `<Foxie>`) under the
/// scene root, so a steering action's agent would resolve to the scene root (no
/// steering components) instead of the agent. Wire the `ActionOf` relationship
/// explicitly: a steering agent's behaviour-tree-root child (a `Repeat`) acts on
/// the agent. (A top-level Rust spawn needs none of this; the agent is the root.)
fn wire_behaviour_agents(
	mut commands: Commands,
	agents: Query<Entity, With<SteerTarget>>,
	children: Query<&Children>,
	roots: Query<(), (With<Repeat>, Without<ActionOf>)>,
) {
	for agent in agents.iter() {
		// the behaviour-tree root is a descendant (the `<Foxie>` slot nests it), not
		// a direct child, so search the whole subtree.
		for entity in children.iter_descendants(agent) {
			if roots.contains(entity) {
				commands.entity(entity).insert(ActionOf(agent));
			}
		}
	}
}

/// A scene `.bsx` builds under the entry's store-root entity (and a `<Scene3d>`
/// slot wrapper), neither of which carries a `Transform`. Bevy's propagation skips
/// any subtree whose chain to the root is broken by a transformless link, so the
/// scene keeps identity `GlobalTransform`s (a camera stuck at the origin renders
/// nothing). Give every transformless entity that has children the spatial
/// components so the whole chain propagates; inert for non-spatial markup.
fn ensure_spatial_roots(
	mut commands: Commands,
	parents: Query<Entity, (With<Children>, Without<Transform>)>,
) {
	for entity in parents.iter() {
		commands
			.entity(entity)
			.insert((Transform::default(), Visibility::default()));
	}
}

#[cfg(feature = "ml")]
pub fn plugin_ml(app: &mut App) {
	use crate::scenes::ml::*;
	app.add_plugins((BeetMlPlugins, FrozenLakePlugin))
		.add_systems(Update, choose_nearest_on_load)
		.register_type::<ChooseNearestOnLoad>()
		// the markup ml scene templates, so `<NearestSentenceAgent/>` /
		// `<SentenceOption/>` / `<ChatSentenceAgent/>` / `<FrozenLake/>` etc resolve
		// in a `.bsx`.
		.register_template::<NearestSentenceAgent>()
		.register_template::<SentenceOption>()
		.register_template::<ChatSentenceAgent>()
		.register_template::<FetchItems>()
		.register_template::<FetchFox>()
		.register_template::<FrozenLake>()
		.register_template::<FrozenLakeRunAgent>()
		.register_template::<FrozenLakeTrainSession>();
}

fn plugin_2d(app: &mut App) {
	app
		.add_systems(Update, follow_cursor_2d)
		// .add_systems(PreUpdate, auto_spawn::auto_spawn.before(PreTickSet))
		.add_systems(Update, randomize_position.in_set(PreTickSet))
		.add_systems(
			Update,
			(update_wrap_around, wrap_around)
			.chain()
			.in_set(PostTickSet),
		)
		.init_resource::<WrapAround>()
		.register_type::<RandomizePosition>()
		.register_type::<WrapAround>()
		.register_type::<FollowCursor2d>()
		// the markup 2d scene templates, so `<Scene2d/>`/`<Sprite2d/>`/`<SpaceScene/>`/
		// `<SeekAgent2d/>`/`<Flock/>` resolve in a `.bsx`.
		.register_template::<Scene2d>()
		.register_template::<Sprite2d>()
		.register_template::<SpaceScene>()
		.register_template::<SeekAgent2d>()
		.register_template::<Flock>()
		/*_*/;
}

fn plugin_3d(app: &mut App) {
	app.add_systems(
			Update,(
			follow_cursor_3d,
			camera_distance,
			rotate_collectables,
			keyboard_controller,
		))
		.register_type::<FollowCursor3d>()
		.register_type::<KeyboardController>()
		.register_type::<CameraDistance>()
		// the markup scene templates, so `<AppWindow/>`/`<UiTerminal/>`/`<Scene3d/>`/
		// `<Lighting3d/>`/`<Ground3d/>` resolve in a `.bsx`.
		.register_template::<AppWindow>()
		.register_template::<UiTerminal>()
		.register_template::<Scene3d>()
		.register_template::<WorldScene>()
		.register_template::<Foxie>()
		.register_template::<SteerTo>()
		.register_template::<AgentOf>()
		.register_template::<Lighting3d>()
		.register_template::<Ground3d>()
		.register_template::<Camera3dLookAt>()
		.register_template::<IkArm>()
		/*-*/;
}
fn assert_local_assets() {
	#[cfg(target_arch = "wasm32")]
	return;
	#[allow(unreachable_code)]
	if !std::path::Path::new("assets/README.md").exists() {
		panic!(
			r#"
🌱🌱🌱

Welcome! This Beet example uses large assets that are stored remotely.
Until bevy supports http asset sources these must be downloaded manually:

Unix:

curl -o ./assets.tar.gz https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
tar -xzvf ./assets.tar.gz
rm ./assets.tar.gz

Windows:

1. Download https://bevyhub-public.s3.us-west-2.amazonaws.com/assets.tar.gz
2. Unzip into `./assets`

🌱🌱🌱
"#
		);
	}
}

/// Toggles fullscreen mode when F11 is pressed.
fn toggle_fullscreen(
	input: When<Res<ButtonInput<KeyCode>>>,
	mut windows: Populated<&mut Window>,
) {
	use bevy::window::WindowMode;
	if input.just_pressed(KeyCode::F11) {
		for mut window in windows.iter_mut() {
			window.mode = match window.mode {
				WindowMode::Windowed => {
					WindowMode::BorderlessFullscreen(MonitorSelection::Current)
				}
				_ => WindowMode::Windowed,
			};
		}
	}
}
