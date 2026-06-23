//! The winit render-path runtime: window lifecycle plus an env-var screenshot
//! hook for headless verification. Added by [`BeetPlugins`](crate::prelude::BeetPlugins)
//! when the `winit` feature links the windowed render stack.
use crate::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy::render::view::screenshot::save_to_disk;
use bevy::winit::WinitSettings;

/// Window lifecycle for the winit render path. `BeetPlugins` boots winit with
/// `primary_window: None` + `ExitCondition::DontExit` (the window is spawned by the
/// loaded scene, eg `<Window/>`), so this:
/// - forces continuous updates ([`WinitSettings::continuous`]) so async asset loads
///   and a no-input scene keep advancing even while the window is unfocused,
/// - writes `AppExit` once the last window closes (`DontExit` suppresses bevy's own),
/// - exits on the escape key, the conventional close affordance.
///
/// It also installs the inert [`screenshot_verify_plugin`].
pub fn render_window_plugin(app: &mut App) {
	app.insert_resource(WinitSettings::continuous())
		.add_plugins(screenshot_verify_plugin)
		.add_systems(Update, (exit_on_esc, exit_when_all_windows_closed));
}

/// Escape ends the app, the conventional close affordance for a windowed example.
fn exit_on_esc(
	input: Res<ButtonInput<KeyCode>>,
	mut exit: MessageWriter<AppExit>,
) {
	if input.just_pressed(KeyCode::Escape) {
		exit.write(AppExit::Success);
	}
}

/// `DontExit` keeps a windowless app (eg a headless server `.bsx`) alive, so the
/// all-windows-closed exit is driven here: once a window has existed and none
/// remain, end the app. `existed` guards the never-had-a-window (server) case.
fn exit_when_all_windows_closed(
	windows: Query<(), With<Window>>,
	mut existed: Local<bool>,
	mut exit: MessageWriter<AppExit>,
) {
	if !windows.is_empty() {
		*existed = true;
	} else if *existed {
		exit.write(AppExit::Success);
	}
}

/// A screenshot verification hook, inert unless `BEET_SCREENSHOT` names an output
/// PNG. When set, it captures the first window's contents on frame
/// `BEET_SCREENSHOT_FRAME` (default 30), saves the PNG, and exits once written. So
/// any windowed beet scene is verifiable headlessly:
/// `BEET_SCREENSHOT=/tmp/x.png beet --main=scene.bsx`.
pub fn screenshot_verify_plugin(app: &mut App) {
	let Ok(path) = env_ext::var("BEET_SCREENSHOT") else {
		return;
	};
	let frame: u32 = env_ext::var("BEET_SCREENSHOT_FRAME")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(30);
	app.insert_resource(ScreenshotVerify { path, frame })
		.add_systems(Update, capture_screenshot);
}

#[derive(Resource)]
struct ScreenshotVerify {
	path: String,
	frame: u32,
}

// Spawn the capture once, on the first frame >= target where a window exists (the
// scene's `<Window/>` arrives a few frames in, after the async entry build).
// `save_to_disk` writes the PNG synchronously when `ScreenshotCaptured` fires; the
// sibling observer queues `AppExit`, processed after the write completes.
fn capture_screenshot(
	mut frame: Local<u32>,
	mut done: Local<bool>,
	config: Res<ScreenshotVerify>,
	windows: Query<Entity, With<Window>>,
	mut commands: Commands,
) {
	*frame += 1;
	if *done || *frame < config.frame {
		return;
	}
	let Some(window) = windows.iter().next() else {
		return;
	};
	*done = true;
	commands
		.spawn(Screenshot::window(window))
		.observe(save_to_disk(config.path.clone()))
		.observe(
			|_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
				exit.write(AppExit::Success);
			},
		);
}
