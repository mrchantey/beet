use std::fmt::Display;

use nu_ansi_term::Color;
use nu_ansi_term::Style;


static ENABLED: std::sync::atomic::AtomicBool =
	std::sync::atomic::AtomicBool::new(true);

/// Temporarily set paint enabled/disabled, restoring previous state on drop
pub struct SetPaintEnabledTemp(bool);

impl SetPaintEnabledTemp {
	pub fn new(next: bool) -> Self {
		let prev = is_enabled();
		set_enabled(next);
		Self(prev)
	}
}

impl Drop for SetPaintEnabledTemp {
	fn drop(&mut self) { set_enabled(self.0); }
}


pub fn set_enabled(enabled: bool) {
	ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
	ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn maybe_paint(style: Style, val: impl Display) -> String {
	if is_enabled() {
		style.paint(val.to_string()).to_string()
	} else {
		val.to_string()
	}
}

pub fn red(val: impl Display) -> String {
	maybe_paint(Color::Red.normal(), val)
}
pub fn green(val: impl Display) -> String {
	maybe_paint(Color::Green.normal(), val)
}
