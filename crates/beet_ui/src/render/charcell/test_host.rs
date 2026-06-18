//! In-process test harness for the live charcell TUI.
//!
//! Drives a [`CharcellTuiPlugin`] app through a [`ChannelTerminal`] instead of a
//! real tty: input bytes are pushed with [`send_input`], the painted frame is
//! snapshotted with [`frame_plain`], and the raw output stream with
//! [`frame_ansi`]. This is the deterministic, CI-friendly verification path
//! reused by every interaction task (08-13). See `context.md` "Verification
//! recipe".
//!
//! `StdioTerminal` reads `/dev/tty`, so a stdin pipe would not reach it; the
//! channel terminal is the only way to feed input in a test.
#![cfg(test)]

use super::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// A booted live TUI app plus the host entity carrying its terminal and buffer.
///
/// Build one with [`TestHost::new`], spawn content under [`host`](Self::host),
/// then drive it with [`send_input`](Self::send_input) / [`step`](Self::step)
/// and assert on [`frame_plain`](Self::frame_plain) /
/// [`frame_ansi`](Self::frame_ansi). The input/output drivers are reused by the
/// interaction tasks (08-13).
pub(crate) struct TestHost {
	pub app: App,
	pub host: Entity,
}

// some drivers (send_input, frame_ansi) are first used by the input tasks (08+);
// they are part of the shared harness API now so they live here from the start.
#[allow(dead_code)]
impl TestHost {
	/// Default test viewport: small enough for legible snapshots, large enough
	/// for multi-line content.
	pub const DEFAULT_SIZE: UVec2 = UVec2::new(40, 12);

	/// Boot a [`CharcellTuiPlugin`] app at [`DEFAULT_SIZE`](Self::DEFAULT_SIZE).
	pub fn new() -> Self { Self::sized(Self::DEFAULT_SIZE) }

	/// Boot a [`CharcellTuiPlugin`] app with a `size`-cell host buffer and a
	/// paired [`ChannelTerminal`] + [`Terminal`] for in-process IO.
	pub fn sized(size: UVec2) -> Self {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CharcellTuiPlugin));

		// the host entity stands in for a window surface: it carries the channel
		// terminal, its paired Terminal, and the DoubleBuffer the pipeline paints.
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((channel, terminal, DoubleBuffer::new(size)))
			.id();
		// the host carries its own Pointer (required by Terminal); run one update
		// to settle Startup before any stepping.
		app.update();
		Self { app, host }
	}

	/// Spawn `bundle` as the content tree under the host buffer entity.
	///
	/// The bundle is attached to the host so the charcell pipeline lays it out
	/// into the host's [`DoubleBuffer`]. Returns the spawned content's relation to
	/// the host (the host itself, which now owns the tree).
	pub fn spawn_content(&mut self, bundle: impl Bundle) {
		self.app.world_mut().entity_mut(self.host).insert(bundle);
	}

	/// Push raw input bytes into the channel terminal's reader (keys, SGR mouse,
	/// paste, resize escapes). Read by the input systems on the next
	/// [`step`](Self::step).
	pub fn send_input(&mut self, data: &[u8]) {
		self.app
			.world_mut()
			.get_mut::<ChannelTerminal>(self.host)
			.unwrap()
			.send_input(data)
			.unwrap();
	}

	/// Advance one frame: runs `Update` then the `PostParseTree` repaint.
	pub fn step(&mut self) { self.app.update(); }

	/// The on-screen frame as plain text (no ANSI), the visual snapshot.
	///
	/// Reads the [front buffer](DoubleBuffer::front_buffer): the live host paints
	/// into the back buffer then swaps, so after a [`step`](Self::step) the
	/// rendered frame is the front one.
	pub fn frame_plain(&self) -> String {
		self.app
			.world()
			.get::<DoubleBuffer>(self.host)
			.unwrap()
			.front_buffer()
			.render_plain()
	}

	/// Drain the raw ANSI output stream written to the terminal since the last
	/// drain, for asserting on the emitted escape sequences.
	pub fn frame_ansi(&mut self) -> Vec<u8> {
		self.app
			.world_mut()
			.get_mut::<ChannelTerminal>(self.host)
			.unwrap()
			.drain_write()
	}

	/// Resize the host buffer, as a terminal resize would.
	pub fn resize(&mut self, size: UVec2) {
		self.app
			.world_mut()
			.get_mut::<DoubleBuffer>(self.host)
			.unwrap()
			.resize(size);
	}

	/// All messages of type `M` currently buffered, for asserting on the input the
	/// bridge emitted this frame (bevy double-buffers messages, so they survive one
	/// frame past the write).
	pub fn messages<M: Message + Clone>(&self) -> Vec<M> {
		self.app
			.world()
			.resource::<Messages<M>>()
			.iter_current_update_messages()
			.cloned()
			.collect()
	}

	/// The host buffer's current size.
	pub fn size(&self) -> UVec2 {
		self.app
			.world()
			.get::<DoubleBuffer>(self.host)
			.unwrap()
			.current_buffer()
			.size()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn boots_and_paints_a_tree() {
		let mut host = TestHost::new();
		host.spawn_content(rsx! {
			<div><h1>"Hi"</h1><p>"Body"</p></div>
		});
		host.step();
		let frame = host.frame_plain();
		frame.as_str().xpect_contains("Hi");
		frame.xpect_contains("Body");
	}

	#[beet_core::test]
	fn resize_reflows_without_panic() {
		let mut host = TestHost::sized(UVec2::new(40, 6));
		host.spawn_content(rsx! {
			<p>"the quick brown fox jumps over the lazy dog"</p>
		});
		host.step();
		// a narrow buffer wraps the sentence onto more rows than a wide one.
		host.resize(UVec2::new(12, 12));
		host.step();
		let narrow = host.frame_plain();
		narrow.as_str().xpect_contains("quick");
		// the long sentence cannot fit on one 12-wide row, so it wraps.
		narrow.lines().count().xpect_greater_or_equal_to(2);
	}
}
