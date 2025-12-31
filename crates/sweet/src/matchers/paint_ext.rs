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

// Basic colors
pub fn red(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Red), val)
}
pub fn green(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Green), val)
}
pub fn cyan(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Cyan), val)
}
pub fn yellow(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Yellow), val)
}
pub fn blue(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Blue), val)
}

// Style modifiers
pub fn dimmed(val: impl Display) -> String {
	maybe_paint(Style::new().dimmed(), val)
}
pub fn bold(val: impl Display) -> String {
	maybe_paint(Style::new().bold(), val)
}

// Bold colors
pub fn red_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Red).bold(), val)
}
pub fn green_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Green).bold(), val)
}
pub fn cyan_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Cyan).bold(), val)
}
pub fn yellow_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Yellow).bold(), val)
}
pub fn blue_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Blue).bold(), val)
}

// Underline bold colors
pub fn cyan_bold_underline(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Cyan).bold().underline(), val)
}
pub fn red_bold_underline(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Red).bold().underline(), val)
}

// Background colors with black text (for badges like PASS/FAIL)
pub fn bg_green_black_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Black).on(Color::Green).bold(), val)
}
pub fn bg_yellow_black_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Black).on(Color::Yellow).bold(), val)
}
pub fn bg_red_black_bold(val: impl Display) -> String {
	maybe_paint(Style::new().fg(Color::Black).on(Color::Red).bold(), val)
}

// Fixed color backgrounds (for diff highlighting)
pub fn green_on_fixed(val: impl Display, bg: u8) -> String {
	maybe_paint(
		Style::new().fg(Color::Green).on(Color::Fixed(bg)).bold(),
		val,
	)
}
pub fn red_on_fixed(val: impl Display, bg: u8) -> String {
	maybe_paint(Style::new().fg(Color::Red).on(Color::Fixed(bg)).bold(), val)
}
