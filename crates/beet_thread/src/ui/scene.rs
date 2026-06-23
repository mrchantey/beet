//! Charcell host shells that wrap a thread's agnostic UI for a local terminal.
//!
//! The host (a [`StdioTerminal`] paired with a [`DoubleBuffer`]) is supplied
//! here, not baked into the view, so the same [`ThreadView`]/[`ThreadComposer`]
//! also serve a per-connection server surface. From markup, nest the view +
//! composer (bound to a `bx:ref` thread) as the host's children:
//!
//! ```rsx
//! <TuiThreadChat>
//!   <div {(ThreadView, OfThread($thread))}/>
//!   <div {(ThreadComposer, OfThread($thread))}/>
//! </TuiThreadChat>
//! ```

use beet_core::prelude::*;
use beet_ui::prelude::style::LayoutStyle;
use beet_ui::prelude::*;

/// A full-screen charcell host for an interactive chat: the alt-screen terminal
/// the transcript + composer paint into. Nest a [`ThreadView`] and
/// [`ThreadComposer`] as its children. Used by the interactive and auto-loop
/// examples (the process runs until Ctrl+C).
#[template]
pub fn TuiThreadChat() -> impl Bundle {
	rsx! {
		<div {(
			StdioTerminal::default(),
			DoubleBuffer::default(),
			LayoutStyle::flex_col(),
		)}>
			<Slot/>
		</div>
	}
}

/// An inline charcell host for a finite, non-interactive run: it keeps the
/// rendered transcript in the terminal scrollback after the process exits. Nest a
/// [`ThreadView`] as its child (no composer). Used by the auto/finite examples.
///
/// Logs redirect to a file (frames paint to `/dev/tty`) so a verbose binary's
/// request tracing never interleaves with the transcript; the normal screen
/// buffer keeps the transcript in scrollback after exit.
#[template]
pub fn TuiThreadTranscript() -> impl Bundle {
	rsx! {
		<div {(
			StdioTerminal::inline()
				.with_log_file(Some(std::path::PathBuf::from("target/beet-log.txt"))),
			DoubleBuffer::default(),
			LayoutStyle::flex_col(),
		)}>
			<Slot/>
		</div>
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_ui::prelude::style::LayoutStyle;
	use beet_ui::prelude::*;
	use bevy::math::UVec2;

	/// Replicates `beet_ui`'s `TestHost` (which is `pub(crate)`): a headless
	/// charcell app whose host entity carries the channel terminal and the
	/// [`DoubleBuffer`] the pipeline paints. Returns `(app, host)`.
	fn charcell_app() -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CharcellTuiPlugin))
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();
		let (channel, terminal) = ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((channel, terminal, DoubleBuffer::new(UVec2::new(40, 12))))
			.id();
		// settle Startup before any content is attached
		app.update();
		(app, host)
	}

	/// The on-screen frame as plain text, the visual snapshot (front buffer, as
	/// the live host paints to the back buffer then swaps).
	fn frame_plain(app: &App, host: Entity) -> String {
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.front_buffer()
			.render_plain()
	}

	/// End to end: calling the thread runs the agent's turn (its `Sequence`
	/// child), which projects into the view's document and renders as charcell
	/// text, the agent's streamed echo included.
	#[beet_core::test]
	async fn renders_window_posts() {
		let (mut app, host) = charcell_app();

		// author an ephemeral thread; the user seed gives the agent something to
		// echo, the Sequence makes calling the thread run the agent's turn
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();

		// attach the reactive view under the charcell host, then kick the thread
		app.world_mut()
			.entity_mut(host)
			.insert(ThreadView::new(thread));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));

		// pump: agent turn -> projection -> document sync -> rows -> charcell paint
		for _ in 0..40 {
			app.update();
		}

		// both rows render: author label + body, with the agent's streamed echo
		// flowing through the per-row FieldRef binding
		let frame = frame_plain(&app, host);
		frame.as_str().xpect_contains("User: hello");
		frame.xpect_contains("Agent: you said: hello");
	}

	/// The chat layout (a [`ThreadView`] and a [`ThreadComposer`] as siblings
	/// under the host) still renders the transcript, so the view works as a child
	/// alongside the input, not only on the host.
	#[beet_core::test]
	async fn chat_layout_renders_transcript() {
		let (mut app, host) = charcell_app();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();
		app.world_mut().entity_mut(host).insert((
			LayoutStyle::flex_col(),
			children![ThreadView::new(thread), ThreadComposer::new(thread)],
		));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		for _ in 0..40 {
			app.update();
		}
		let frame = frame_plain(&app, host);
		frame.as_str().xpect_contains("User: hello");
		frame.xpect_contains("Agent: you said: hello");
	}

	/// The user's turn is a Sequence action: calling the thread reaches the
	/// `User` actor's [`UserInput`], which waits for the composer's [`Submit`],
	/// appends the typed post, then passes so the agent replies to it. The
	/// `Submit` is fired directly here (the focus/typing path is `beet_ui`'s); a
	/// full keystroke run is `keyboard_submit_drives_reply`.
	#[beet_core::test]
	async fn user_input_advances_on_submit() {
		let (mut app, host) = charcell_app();

		// user turn first, then the agent: one Sequence call exercises both
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(Actor::user(), UserInput),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();

		// mount the chat UI so the composer's <form> exists for the turn to await
		app.world_mut().entity_mut(host).insert((
			LayoutStyle::flex_col(),
			children![ThreadView::new(thread), ThreadComposer::new(thread)],
		));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));

		// pump until the user turn has installed its Submit observer
		for _ in 0..20 {
			app.update();
		}

		// the user ends their turn by submitting "hello" on the composer's <form>
		let form = app
			.world_mut()
			.with_state::<ElementQuery, _>(|elements| {
				elements
					.iter()
					.find(|view| view.tag() == "form")
					.map(|view| view.entity)
			})
			.unwrap();
		let values = Value::Map(
			[("message".into(), Value::new("hello"))]
				.into_iter()
				.collect(),
		);
		app.world_mut().trigger(Submit { form, values });

		// pump: user post appended -> Sequence advances -> agent replies -> paint
		for _ in 0..40 {
			app.update();
		}

		let frame = frame_plain(&app, host);
		frame.as_str().xpect_contains("User: hello");
		frame.xpect_contains("Agent: you said: hello");
	}

	/// The full deterministic interaction: real keystrokes through the input
	/// bridge type into the focused composer and Enter submits, advancing the
	/// user turn so the mock agent replies. The charcell host is wired by
	/// [`focus_chat_composer`] exactly as the local `TuiThreadChat` wires the real one.
	#[beet_core::test]
	async fn keyboard_submit_drives_reply() {
		let (mut app, host) = charcell_app();

		// user turn first, then the agent, so one exchange yields a reply
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(Actor::user(), UserInput),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();

		// mount the chat UI on the host (as the local `TuiThreadChat` does), then kick
		app.world_mut().entity_mut(host).insert((
			LayoutStyle::flex_col(),
			children![ThreadView::new(thread), ThreadComposer::new(thread)],
		));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));

		// pump: composer builds + focuses, the user turn installs its observer
		for _ in 0..25 {
			app.update();
		}

		// type "hello" + Enter through the real terminal input bridge
		app.world_mut()
			.get_mut::<ChannelTerminal>(host)
			.unwrap()
			.send_input(b"hello\r")
			.unwrap();
		for _ in 0..40 {
			app.update();
		}

		let frame = frame_plain(&app, host);
		frame.as_str().xpect_contains("User: hello");
		frame.xpect_contains("Agent: you said: hello");
	}
}
