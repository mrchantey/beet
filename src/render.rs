//! The winit render-path runtime: window lifecycle plus an env-var screenshot
//! hook for headless verification. Added by [`BeetPlugins`](crate::prelude::BeetPlugins)
//! when the `winit` feature links the windowed render stack.
use crate::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy::render::view::screenshot::save_to_disk;
use bevy::winit::EventLoopProxyWrapper;
use bevy::winit::WinitSettings;
use bevy::winit::WinitUserEvent;

/// Window lifecycle for the winit render path. `BeetPlugins` boots winit windowless
/// (`primary_window: None` + `DontExit`), so this:
/// - keeps the event loop ticking even with no window ([`keep_event_loop_awake`]),
///   the fix that lets a data-spawned `<AppWindow/>` ever appear and a headless
///   `.bsx` keep running under the render binary,
/// - forces continuous updates so async asset loads and a no-input scene advance
///   even while the window is unfocused (a screenshot harness rarely holds focus),
/// - exits on the escape key, and once a window has existed and all have closed.
///
/// It also installs the inert [`screenshot_verify_plugin`].
pub fn render_window_plugin(app: &mut App) {
	app.insert_resource(WinitSettings::continuous())
		.add_plugins(screenshot_verify_plugin)
		.add_systems(Startup, keep_event_loop_awake)
		.add_systems(Update, (exit_on_esc, exit_when_all_windows_closed));
}

/// Keep the winit event loop ticking even with no window. On Wayland/X11 bevy parks
/// the loop in `ControlFlow::Wait` in Continuous mode and only wakes it via a
/// per-window redraw request (`bevy_winit` state.rs), so with `primary_window: None`
/// and no window yet the app would never tick to build a window-spawning scene (or
/// run a headless server). A background thread pings the event-loop proxy at ~30Hz;
/// it ends when the loop closes (the send errors), so the app still exits cleanly.
fn keep_event_loop_awake(proxy: Res<EventLoopProxyWrapper>) {
	let proxy = (**proxy).clone();
	std::thread::spawn(move || {
		while proxy.send_event(WinitUserEvent::WakeUp).is_ok() {
			std::thread::sleep(Duration::from_millis(33));
		}
	});
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
	info!("screenshot harness armed: path={path}, capture frame={frame}");
	app.insert_resource(ScreenshotVerify { path, frame })
		.add_systems(Update, (capture_screenshot, screenshot_timeout));
}

// safety net: exit a few seconds past the target frame even if the capture never
// completes, so a run always terminates (and flushes logs) instead of hanging.
fn screenshot_timeout(
	mut frame: Local<u32>,
	config: Res<ScreenshotVerify>,
	mut exit: MessageWriter<AppExit>,
) {
	*frame += 1;
	if *frame > config.frame + 240 {
		warn!(
			"screenshot harness: timed out at frame {} with no capture, exiting",
			*frame
		);
		exit.write(AppExit::Success);
	}
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
	info!("screenshot harness: capturing -> {}", config.path);
	commands
		.spawn(Screenshot::window(window))
		.observe(save_to_disk(config.path.clone()))
		.observe(
			|_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
				info!("screenshot harness: captured, exiting");
				exit.write(AppExit::Success);
			},
		);
}
