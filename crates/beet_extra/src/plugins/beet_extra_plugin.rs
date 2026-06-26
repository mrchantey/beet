use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// The capabilities every beet example shares: the action + spatial systems, the
/// 2d/3d helper systems, the terminal UI, and the window conveniences (esc to
/// close, F11 fullscreen). The runner and window are [`BeetPlugins`](beet)' job;
/// this only adds behaviour, so it composes under either runner.
pub fn beet_extra_plugin(app: &mut App) {
	// `ActionPlugin` may already be present (the router stack adds it via
	// `init_plugin` on the CLI render path), so add it idempotently rather than
	// panic on the double-add when this group composes with a router-bearing app.
	app.init_plugin::<ActionPlugin>()
		.add_plugins((
			// drains deferred asset loads (clips, Bert) so a scene's `LoadTemplate`
			// (and its `RunOnLoad` gate) fires once every asset has settled.
			AssetTemplatePlugin,
			BeetSpatialPlugins,
			plugin_2d,
			plugin_3d,
			UiTerminalPlugin,
		))
		.add_systems(
			Update,
			(close_on_esc, toggle_fullscreen, ensure_spatial_roots),
		)
		.init_resource::<RandomSource>()
		// `RunOnLoad` (the behaviour load verb the scenes start their trees with)
		// lives in `beet_net` with the rest of the load-verb family, registered by
		// `ServerPlugin`. A windowed scene need not carry the router stack, so
		// register it here too (idempotent) to keep the render set self-sufficient.
		.register_type::<RunOnLoad>()
		.register_type::<Collectable>();
}

/// A scene `.bsx` builds under the entry's store-root entity, which carries no
/// `Transform`: even though `<Scene3d>` now hosts its transform without a wrapper
/// element, the entry store-root above it is still transformless, so the chain to
/// the scene root has a transformless link. Bevy's propagation skips any subtree
/// broken by such a link, so the scene keeps identity `GlobalTransform`s (a camera
/// stuck at the origin renders nothing). Give every transformless entity that has
/// children the spatial components so the whole chain propagates; inert for
/// non-spatial markup. Kept until a later verification phase confirms scenes render.
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
		.init_resource::<WrapAroundBounds>()
		.register_type::<RandomizePosition>()
		.register_type::<WrapAroundBounds>()
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
		.register_template::<Lighting3d>()
		.register_template::<Ground3d>()
		.register_template::<Camera3dLookAt>()
		.register_template::<IkTarget>()
		/*-*/;
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
