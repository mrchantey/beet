//! Common system functions for Bevy applications.
//!
//! This module provides reusable system functions for common operations
//! like exiting the application.

use crate::prelude::*;
use bevy::app::AppExit;
use bevy::diagnostic::FrameCount;

/// Returns a system that exits the application after the specified number of frames.
pub fn exit_in_frames(
	count: u32,
) -> impl Fn(Res<FrameCount>, MessageWriter<AppExit>) {
	move |frames, mut exit| {
		if frames.0 >= count - 1 {
			exit.write(AppExit::Success);
		}
	}
}

/// Closes the application when the Escape key is pressed.
pub fn close_on_esc(
	input: When<Res<ButtonInput<KeyCode>>>,
	mut exit: MessageWriter<AppExit>,
) {
	if input.just_pressed(KeyCode::Escape) {
		exit.write(AppExit::Success);
	}
}
