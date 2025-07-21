use crate::prelude::When;
use bevy::app::AppExit;
use bevy::diagnostic::FrameCount;
use bevy::prelude::*;

pub fn exit_in_frames(
	count: u32,
) -> impl Fn(Res<FrameCount>, EventWriter<AppExit>) {
	move |frames, mut exit| {
		if frames.0 >= count - 1 {
			exit.write(AppExit::Success);
		}
	}
}

/// Closes the application when the Escape key is pressed.
pub fn close_on_esc(
	input: When<Res<ButtonInput<KeyCode>>>,
	mut exit: EventWriter<AppExit>,
) {
	if input.just_pressed(KeyCode::Escape) {
		exit.write(AppExit::Success);
	}
}

/// Toggles fullscreen mode when F11 is pressed.
#[cfg(feature = "bevy_default")]
pub fn toggle_fullscreen(
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
