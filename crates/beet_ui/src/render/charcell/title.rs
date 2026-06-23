//! Mirroring the document `<title>` onto the terminal window title bar.
//!
//! The browser sets its tab title from the document `<title>`; the terminal has
//! the same affordance through an OSC-0 escape. [`update_terminal_title`] gives
//! the charcell TUI parity: as navigation swaps the page (and its
//! `RouteHead`-bound `<title>`), the terminal's title bar follows.
use super::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// System: reflect the active document's `<title>` text onto the terminal window
/// title bar via an OSC-0 escape, rewriting only when it changes.
///
/// Follows the `apply_*` decoration pattern — it scans elements through
/// [`ElementQuery`] for a `<title>` and reads its inner text — then writes the
/// set-window-title sequence to every [`Terminal`]'s writer (flushed by
/// [`flush_terminals`]). The last value is cached in a [`Local`] so an unchanged
/// title is a no-op, and a document with no `<title>` leaves the bar untouched.
#[cfg(feature = "tui")]
pub fn update_terminal_title(
	elements: ElementQuery,
	mut terminals: Query<&mut Terminal>,
	mut last: Local<Option<String>>,
) -> Result {
	// the first `<title>` element carrying text — the document title.
	let Some(title) = elements.iter().find_map(|view| {
		(view.tag() == "title")
			.then_some(view.inner_text)
			.flatten()
			.and_then(|(_, value)| value.as_str().ok())
			.map(str::to_string)
	}) else {
		return Ok(());
	};
	// strip control chars so the text cannot terminate the OSC sequence early.
	let title: String = title.chars().filter(|c| !c.is_control()).collect();
	if last.as_deref() == Some(title.as_str()) {
		return Ok(());
	}
	for mut terminal in terminals.iter_mut() {
		escape::set_window_title(terminal.writer_mut(), &title)?;
	}
	*last = Some(title);
	Ok(())
}
