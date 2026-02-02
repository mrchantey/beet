//! ANSI paint utilities.
//!
//! This module provides functions for adding ANSI color codes to terminal output.
//! Colors can be globally enabled or disabled using [`set_enabled`].
//!
//! # Example
//!
//! ```
//! use beet_core::prelude::*;
//! let message = paint_ext::green_bold("Success!");
//! println!("{}", message);
//! ```

use std::fmt::Display;

use nu_ansi_term::Color;
use nu_ansi_term::Style;



static ENABLED: std::sync::atomic::AtomicBool =
	std::sync::atomic::AtomicBool::new(true);

/// Temporarily sets paint enabled/disabled, restoring previous state on drop.
pub struct SetPaintEnabledTemp(bool);

impl SetPaintEnabledTemp {
	/// Creates a new guard that sets the paint enabled state.
	pub fn new(next: bool) -> Self {
		let prev = is_enabled();
		set_enabled(next);
		Self(prev)
	}
}

impl Drop for SetPaintEnabledTemp {
	fn drop(&mut self) { set_enabled(self.0); }
}


/// Sets whether ANSI colors are enabled globally.
pub fn set_enabled(enabled: bool) {
	ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Returns whether ANSI colors are currently enabled.
pub fn is_enabled() -> bool {
	ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Applies a style to a value if colors are enabled.
pub fn maybe_paint(style: Style, val: impl Display) -> String {
	if is_enabled() {
		style.paint(val.to_string()).to_string()
	} else {
		val.to_string()
	}
}

/// Paints text red.
pub fn red(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Red), val)
}

/// Paints text green.
pub fn green(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Green), val)
}

/// Paints text cyan.
pub fn cyan(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Cyan), val)
}

/// Paints text yellow.
pub fn yellow(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Yellow), val)
}

/// Paints text blue.
pub fn blue(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Blue), val)
}

/// Paints text with a dimmed style.
pub fn dimmed(val: impl Display) -> String {
	maybe_paint(Style::new().dimmed(), val)
}

/// Paints text bold.
pub fn bold(val: impl Display) -> String {
	maybe_paint(Style::new().bold(), val)
}

/// Paints text red and bold.
pub fn red_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Red).bold(), val)
}

/// Paints text green and bold.
pub fn green_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Green).bold(), val)
}

/// Paints text cyan and bold.
pub fn cyan_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Cyan).bold(), val)
}

/// Paints text yellow and bold.
pub fn yellow_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Yellow).bold(), val)
}

/// Paints text blue and bold.
pub fn blue_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Blue).bold(), val)
}

/// Paints text with black text on green background, bold.
pub fn bg_green_black_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Black).on(Color::Green).bold(), val)
}

/// Paints text with black text on yellow background, bold.
pub fn bg_yellow_black_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Black).on(Color::Yellow).bold(), val)
}

/// Paints text with black text on red background, bold.
pub fn bg_red_black_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Black).on(Color::Red).bold(), val)
}

/// Paints text green on a fixed color background, bold.
pub fn green_on_fixed(val: impl Display, bg: u8) -> String {
	maybe_paint(
		Style::new().fg(Color::Green).on(Color::Fixed(bg)).bold(),
		val,
	)
}

/// Paints text red on a fixed color background, bold.
pub fn red_on_fixed(val: impl Display, bg: u8) -> String {
	maybe_paint(Style::new().fg(Color::Red).on(Color::Fixed(bg)).bold(), val)
}
