//! OSC-52 clipboard writes: set the terminal client's clipboard.
//!
//! The live TUI captures the mouse, so the terminal can't open a clicked link
//! itself, and over SSH the app can't reach the user's machine to open a browser.
//! Instead a clicked external link is copied to the *client's* clipboard via an
//! OSC-52 escape, which terminals honor even over SSH. Mirrors the title flush:
//! an escape written to the surface's [`Terminal`] outside the cell draw, carried
//! out by [`flush_terminals`](super::flush_terminals).

use super::*;
use base64::Engine;
use beet_core::prelude::*;

/// Request to copy `content` to the clipboard of `surface`'s terminal client.
///
/// Fired by the link handler for an external link on a remote (SSH) surface and
/// handled by [`flush_clipboard`], which writes an OSC-52 sequence to that
/// surface's [`Terminal`].
#[derive(Debug, Clone, Message)]
pub struct CopyToClipboard {
	/// The terminal surface whose client clipboard to set.
	pub surface: Entity,
	/// The text to copy.
	pub content: SmolStr,
}

/// Write each [`CopyToClipboard`] to its surface's [`Terminal`] as an OSC-52
/// sequence, base64-encoding the payload per the protocol.
///
/// A surface with no terminal (eg torn down mid-frame) is skipped. The bytes are
/// flushed to the client by [`flush_terminals`](super::flush_terminals), so this
/// runs in the same render chain before it.
pub fn flush_clipboard(
	mut events: MessageReader<CopyToClipboard>,
	mut terminals: Query<&mut Terminal>,
	// toasts are a template-gated widget; without it the copy still happens, just
	// without the on-screen confirmation.
	#[cfg(feature = "template")] mut commands: Commands,
) -> Result {
	for ev in events.read() {
		let Ok(mut terminal) = terminals.get_mut(ev.surface) else {
			continue;
		};
		let encoded = base64::engine::general_purpose::STANDARD
			.encode(ev.content.as_bytes());
		write!(
			terminal.writer_mut(),
			"{}{encoded}{}",
			escape::OSC52_CLIPBOARD,
			escape::ST
		)?;
		// confirm the copy with a transient toast on the same surface.
		#[cfg(feature = "template")]
		crate::prelude::Toast::show(
			&mut commands,
			ev.surface,
			"Copied to clipboard",
		);
	}
	Ok(())
}

#[cfg(all(test, feature = "tui"))]
mod test {
	use super::*;
	use crate::render::charcell::test_host::TestHost;

	/// A [`CopyToClipboard`] for a surface writes an OSC-52 sequence carrying the
	/// base64-encoded content to that surface's terminal.
	#[beet_core::test]
	fn writes_osc52_sequence() {
		let mut host = TestHost::new();
		host.step();
		host.frame_ansi(); // drain the boot frame
		host.app.world_mut().write_message(CopyToClipboard {
			surface: host.host,
			content: "https://example.com".into(),
		});
		host.step();
		// base64("https://example.com")
		let encoded = base64::engine::general_purpose::STANDARD
			.encode("https://example.com");
		String::from_utf8_lossy(&host.frame_ansi())
			.into_owned()
			.xpect_contains(&format!("\x1b]52;c;{encoded}\x1b\\"));
	}

	/// A copy also pops a transient toast on the surface to confirm it.
	#[cfg(feature = "template")]
	#[beet_core::test]
	fn copy_pops_toast() {
		let mut host = TestHost::new();
		host.step();
		host.app.world_mut().write_message(CopyToClipboard {
			surface: host.host,
			content: "https://example.com".into(),
		});
		host.step();
		// a Toast was spawned as a child of the surface.
		host.app
			.world_mut()
			.query_filtered::<&ChildOf, With<crate::prelude::Toast>>()
			.iter(host.app.world())
			.any(|child_of| child_of.parent() == host.host)
			.xpect_true();
	}
}
