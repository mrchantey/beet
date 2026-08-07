//! Common system functions for Bevy applications.
//!
//! This module provides reusable system functions for common operations
//! like exiting the application. Keyboard-gated, since its only system reads
//! keyboard input.

use crate::prelude::*;
use bevy::app::AppExit;

/// Closes the application when the Escape key is pressed.
pub fn close_on_esc(
	input: When<Res<ButtonInput<KeyCode>>>,
	mut exit: MessageWriter<AppExit>,
) {
	if input.just_pressed(KeyCode::Escape) {
		exit.write(AppExit::Success);
	}
}
