use std::fmt::Display;

use nu_ansi_term::Color;


static ENABLED: std::sync::atomic::AtomicBool =
	std::sync::atomic::AtomicBool::new(true);


pub fn set_enabled(enabled: bool) {
	ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
	ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn red(val: impl Display) -> String {
	if is_enabled() {
		Color::Red.paint(val.to_string()).to_string()
	} else {
		val.to_string()
	}
}
pub fn green(val: impl Display) -> String {
	if is_enabled() {
		Color::Green.paint(val.to_string()).to_string()
	} else {
		val.to_string()
	}
}
