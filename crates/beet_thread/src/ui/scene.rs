//! Charcell host shells that wrap a thread's agnostic UI for a local terminal.
//!
//! The host (a [`StdioTerminal`] paired with a [`DoubleBuffer`]) is supplied
//! here, not baked into the view, so the same [`ThreadView`]/[`CreatePostForm`]
//! also serve a per-connection server surface. From markup, nest the view +
//! form (bound to a `bx:ref` thread) as the host's children:
//!
//! ```rsx
//! <TuiThreadChat>
//!   <div {(ThreadView, OfThread($thread))}/>
//!   <div {(CreatePostForm, OfThread($thread))}/>
//! </TuiThreadChat>
//! ```

use beet_core::prelude::*;
use beet_ui::prelude::material::colors;
use beet_ui::prelude::*;

/// Shared shell styling for a charcell thread host: a full-screen column (the
/// render root already fills the viewport) tinted with the surface palette, so the
/// transcript grows and a composer pins to the bottom. Cascade styling
/// (`inline_class!`), since the thread UI's rows are cascade-styled and
/// `resolve_styles` rebuilds every node's `LayoutStyle` from the cascade, which
/// would clobber a set component.
fn host_column() -> impl Bundle {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(style::common_props::FlexDirectionProp, style::Direction::Vertical),
		// stretch children across the full width, so the transcript and the
		// composer (and its top-border separator) span the terminal
		(style::common_props::AlignItemsProp, style::AlignItems::Stretch),
		token(style::common_props::BackgroundColor, colors::Surface),
		token(style::common_props::ForegroundColor, colors::OnSurface),
	]
}

/// A full-screen charcell host for an interactive chat: the alt-screen terminal
/// the transcript + form paint into. Nest a [`ThreadView`] and a
/// [`CreatePostForm`] as its children. Used by the interactive and auto-loop
/// examples (the process runs until Ctrl+C).
///
/// The host *is* its own [`RenderSurface`] (via [`RenderSurface::self_referential`]),
/// so the whole nested subtree (the [`CreatePostForm`], its `<input>`, the
/// [`ThreadView`]) resolves to it through `SurfaceQuery` with no per-widget wiring.
#[template]
pub fn TuiThreadChat() -> impl Bundle {
	rsx! {
		<div {(
			StdioTerminal::default(),
			DoubleBuffer::default(),
			host_column(),
			RenderSurface::self_referential(),
		)}>
			<Slot/>
		</div>
	}
}

/// A full-screen charcell host for a finite, non-interactive run: the alt-screen
/// terminal a transcript paints into, restored to the prior screen when the
/// process exits (a self-exiting program leaves the alternate screen the same way
/// Ctrl+C does). Nest a [`ThreadView`] as its child (no composer). Used by the
/// auto/finite examples.
///
/// Like [`TuiThreadChat`] but without a composer; the alternate screen keeps the
/// run from scrolling the terminal and returns it clean on exit. Logs redirect to
/// a file (frames paint to `/dev/tty`, the default) so request tracing never
/// corrupts the screen.
#[template]
pub fn TuiThreadTranscript() -> impl Bundle {
	rsx! {
		<div {(
			StdioTerminal::default(),
			DoubleBuffer::default(),
			host_column(),
			RenderSurface::self_referential(),
		)}>
			<Slot/>
		</div>
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	// `Thread::id()` / `Actor::id()` / `Actor::kind()` come from this trait; the
	// `beet_ui` glob below otherwise shadows the prelude re-export.
	use crate::table::Table;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_ui::prelude::style::LayoutStyle;
	use beet_ui::prelude::*;
	use bevy::math::UVec2;

	/// Replicates `beet_ui`'s `TestHost` (which is `pub(crate)`): a headless
	/// charcell app whose host entity carries the channel terminal and the
	/// [`DoubleBuffer`] the pipeline paints. The host is its own [`RenderSurface`]
	/// (via [`RenderSurface::self_referential`]), as the local [`TuiThreadChat`] is,
	/// so the nested form subtree resolves to the host.
	/// Returns `(app, host)`.
	async fn charcell_app() -> (App, Entity) {
		charcell_app_sized(UVec2::new(40, 12)).await
	}

	/// As [`charcell_app`], with an explicit buffer size for layout-sensitive
	/// renders (eg the full chat snapshot).
	async fn charcell_app_sized(size: UVec2) -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CharcellTuiPlugin))
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((
				// a `<div>` like the real `TuiThreadChat` host, so cascade-resolved
				// layout (element-only in `resolve_styles`) applies to it
				Element::new("div"),
				channel,
				terminal,
				DoubleBuffer::new(size),
				RenderSurface::self_referential(),
			))
			.id();
		// settle Startup + the async `CreatePostForm` registration (seeded into an
		// in-memory store, loaded by `ThreadUiPlugin`'s `TemplateDir`) before any
		// content is attached, so a form spawned below resolves it deterministically.
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
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

	/// The first rendered element with `tag`, eg the form's `<form>` or
	/// `<input>` built from `CreatePostForm.bsx`.
	fn element_by_tag(app: &mut App, tag: &str) -> Option<Entity> {
		app.world_mut().with_state::<ElementQuery, _>(|elements| {
			elements
				.iter()
				.find(|view| view.tag() == tag)
				.map(|view| view.entity)
		})
	}

	/// End to end: calling the thread runs the agent's turn (its `Sequence`
	/// child), which projects into the view's document and renders as charcell
	/// text, the agent's streamed echo included.
	#[beet_core::test]
	async fn renders_window_posts() {
		let (mut app, host) = charcell_app().await;

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
		// author label and body render on their own lines now, role-styled
		frame.as_str().xpect_contains("User");
		frame.as_str().xpect_contains("hello");
		frame.as_str().xpect_contains("Agent");
		frame.as_str().xpect_contains("you said: hello");
	}

	/// Push an error post (5xx intent) authored by the thread's agent into its
	/// window, so the transcript snapshot exercises the `error` role styling
	/// without a failing network call.
	fn push_error_post(app: &mut App, thread: Entity) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let agent_id = app
			.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.actors()
			.values()
			.find(|actor| actor.kind() == ActorKind::Agent)
			.unwrap()
			.id();
		app.world_mut()
			.get_mut::<ThreadWindow>(thread)
			.unwrap()
			.upsert_post(AgentPost::new_error(
				agent_id,
				thread_id,
				"model request failed: 401 Unauthorized",
				PostStatus::Completed,
			));
	}

	/// The full chat surface (scrollable transcript + composer) renders every
	/// role with its own styling. A static thread (seed posts, no streamers) plus
	/// a pushed error post keeps the snapshot deterministic and offline.
	#[beet_core::test]
	async fn chat_surface_snapshot() {
		let (mut app, host) = charcell_app_sized(UVec2::new(56, 24)).await;
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(
					Actor::new("System", ActorKind::System),
					children![Post::spawn("you are a friendly robot")],
				),
				(
					Actor::new("Billy", ActorKind::User),
					children![Post::spawn("hello there robot")],
				),
				(
					Actor::new("BeepBot", ActorKind::Agent),
					children![Post::spawn("Beep boop! Greetings, human.")],
				),
			]))
			.id();
		app.update();
		ThreadWindow::reduce_now(app.world_mut());
		push_error_post(&mut app, thread);
		app.world_mut().entity_mut(host).insert((
			super::host_column(),
			children![ThreadView::new(thread), CreatePostForm::new(thread)],
		));
		for _ in 0..30 {
			app.update();
		}
		frame_plain(&app, host).xpect_snapshot();
	}

	/// Append numbered text posts (`line NN`) authored by the thread's agent over
	/// `lines`, so a transcript can be grown past its viewport.
	fn push_numbered_posts(
		app: &mut App,
		thread: Entity,
		lines: std::ops::RangeInclusive<usize>,
	) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let agent_id = app
			.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.actors()
			.values()
			.find(|actor| actor.kind() == ActorKind::Agent)
			.unwrap()
			.id();
		for index in lines {
			app.world_mut()
				.get_mut::<ThreadWindow>(thread)
				.unwrap()
				.upsert_post(AgentPost::new_text(
					agent_id,
					thread_id,
					format!("line {index:02}"),
					PostStatus::Completed,
				));
		}
	}

	/// A transcript taller than its viewport scrolls *inside its own region*
	/// rather than pushing the composer off screen: the composer stays visible,
	/// and `follow_thread_scroll` pins the view to the latest post (the earliest
	/// are clipped out the top). Regression for charcell internal scroll, where a
	/// `flex-grow` + `overflow-y: auto` child used to grow to its full content
	/// height and shove later flex items past the screen.
	#[beet_core::test]
	async fn chat_scroll_keeps_composer_visible() {
		let (mut app, host) = charcell_app_sized(UVec2::new(48, 16)).await;
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(
					Actor::new("System", ActorKind::System),
					children![Post::spawn("be brief")],
				),
				(Actor::new("Agent", ActorKind::Agent),),
			]))
			.id();
		app.update();
		ThreadWindow::reduce_now(app.world_mut());
		// far more posts than the 16-row viewport can hold
		push_numbered_posts(&mut app, thread, 1..=14);
		app.world_mut().entity_mut(host).insert((
			super::host_column(),
			children![ThreadView::new(thread), CreatePostForm::new(thread)],
		));
		for _ in 0..30 {
			app.update();
		}
		let frame = frame_plain(&app, host);
		// the composer survived (the transcript clipped instead of pushing it off) ...
		frame.as_str().xpect_contains("Send");
		// ... the latest post is pinned into view ...
		frame.as_str().xpect_contains("line 14");
		// ... and the earliest is scrolled out of the clipped region.
		frame.as_str().xnot().xpect_contains("line 01");
		frame.xpect_snapshot();
	}

	/// Follow-to-bottom only sticks while the reader is at the bottom: once they
	/// scroll up, a new post does not yank them back down. Regression for
	/// `follow_thread_scroll` wrestling the scroll from the user.
	#[beet_core::test]
	async fn follow_leaves_scrolled_up_reader() {
		let (mut app, host) = charcell_app_sized(UVec2::new(48, 16)).await;
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(
					Actor::new("System", ActorKind::System),
					children![Post::spawn("be brief")],
				),
				(Actor::new("Agent", ActorKind::Agent),),
			]))
			.id();
		app.update();
		ThreadWindow::reduce_now(app.world_mut());
		push_numbered_posts(&mut app, thread, 1..=12);
		app.world_mut()
			.entity_mut(host)
			.insert((super::host_column(), children![ThreadView::new(thread)]));
		for _ in 0..30 {
			app.update();
		}
		// the view auto-followed to the bottom; scroll the reader back to the top
		let scroll = app
			.world_mut()
			.query_filtered::<Entity, With<ThreadScroll>>()
			.single(app.world())
			.unwrap();
		app.world_mut()
			.get_mut::<beet_ui::prelude::ScrollPosition>(scroll)
			.unwrap()
			.offset
			.y = 0;
		for _ in 0..5 {
			app.update();
		}
		// a new post arrives: the scrolled-up reader is left in place, not yanked
		push_numbered_posts(&mut app, thread, 13..=13);
		for _ in 0..15 {
			app.update();
		}
		let pos = app
			.world()
			.get::<beet_ui::prelude::ScrollPosition>(scroll)
			.unwrap();
		(pos.offset.y < pos.max.y).xpect_true();
		// still showing the top, not pinned to the latest
		frame_plain(&app, host).as_str().xpect_contains("line 01");
	}

	/// The chat layout (a [`ThreadView`] and a [`CreatePostForm`] as siblings
	/// under the host) still renders the transcript, so the view works as a child
	/// alongside the input, not only on the host.
	#[beet_core::test]
	async fn chat_layout_renders_transcript() {
		let (mut app, host) = charcell_app().await;
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
			children![ThreadView::new(thread), CreatePostForm::new(thread)],
		));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		for _ in 0..40 {
			app.update();
		}
		let frame = frame_plain(&app, host);
		// author label and body render on their own lines now, role-styled
		frame.as_str().xpect_contains("User");
		frame.as_str().xpect_contains("hello");
		frame.as_str().xpect_contains("Agent");
		frame.as_str().xpect_contains("you said: hello");
	}

	/// The user's turn is a Sequence action: calling the thread reaches the
	/// `User` actor's [`UserInput`], which waits for the composer's [`Submit`],
	/// appends the typed post, then passes so the agent replies to it. The
	/// `Submit` is fired directly here (the focus/typing path is `beet_ui`'s); a
	/// full keystroke run is `keyboard_submit_drives_reply`.
	#[beet_core::test]
	async fn user_input_advances_on_submit() {
		let (mut app, host) = charcell_app().await;

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
			children![ThreadView::new(thread), CreatePostForm::new(thread)],
		));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));

		// pump until the user turn has installed its Submit observer
		for _ in 0..20 {
			app.update();
		}

		// the user ends their turn by submitting "hello" on the composer's <form>
		let form = element_by_tag(&mut app, "form").unwrap();
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
		// author label and body render on their own lines now, role-styled
		frame.as_str().xpect_contains("User");
		frame.as_str().xpect_contains("hello");
		frame.as_str().xpect_contains("Agent");
		frame.as_str().xpect_contains("you said: hello");
	}

	/// The full deterministic interaction: real keystrokes through the input
	/// bridge type into the focused form and Enter submits, advancing the user
	/// turn so the mock agent replies. The host carries `RenderSurface(self)` so
	/// the form's subtree resolves to it, and `{FocusOnAdd}` focuses its
	/// `<input>`, exactly as the local `TuiThreadChat` wires the real one.
	#[beet_core::test]
	async fn keyboard_submit_drives_reply() {
		let (mut app, host) = charcell_app().await;

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
			children![ThreadView::new(thread), CreatePostForm::new(thread)],
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
		// author label and body render on their own lines now, role-styled
		frame.as_str().xpect_contains("User");
		frame.as_str().xpect_contains("hello");
		frame.as_str().xpect_contains("Agent");
		frame.as_str().xpect_contains("you said: hello");
	}

	/// The widget builds its `<form>` from `CreatePostForm.bsx` (interim-loaded
	/// into the `BsxTemplateRegistry`), not inline rsx: mounting one yields a
	/// rendered `<form>` with the `message` `<input>` and the `Send` `<button>`.
	#[beet_core::test]
	async fn composer_renders_from_bsx() {
		let (mut app, host) = charcell_app().await;
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new()))
			.id();
		app.update();
		app.world_mut()
			.entity_mut(host)
			.insert(children![CreatePostForm::new(thread)]);
		for _ in 0..25 {
			app.update();
		}
		element_by_tag(&mut app, "form").is_some().xpect_true();
		element_by_tag(&mut app, "input").is_some().xpect_true();
		element_by_tag(&mut app, "button").is_some().xpect_true();
	}

	/// Focus + surface scoping survive an extra wrapper element between the host
	/// and the form: the host carries `RenderSurface(self)`, so the form's whole
	/// subtree resolves to it through `SurfaceQuery` regardless of depth, and
	/// `{FocusOnAdd}` focuses its `<input>` (no per-widget surface wiring, so no
	/// fixed-depth walk to break).
	#[beet_core::test]
	async fn composer_focus_survives_wrapper() {
		let (mut app, host) = charcell_app().await;
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new()))
			.id();
		app.update();
		// nest the form under an extra <div> wrapper under the host, not
		// directly, so any fixed-depth walk would miss the host.
		app.world_mut().entity_mut(host).insert((
			LayoutStyle::flex_col(),
			children![(Element::new("div"), children![CreatePostForm::new(
				thread
			)])],
		));
		// settle the build + focus deterministically rather than spinning frames
		AsyncRunner::settle_async_tasks(app.world_mut()).await;

		// the form's <input> ended up focused ...
		let input = element_by_tag(&mut app, "input").unwrap();
		app.world().entity(input).contains::<Focus>().xpect_true();
		// ... and its resolved surface is the host (so typed bytes route to it).
		app.world_mut()
			.with_state::<SurfaceQuery, _>(|surfaces| {
				surfaces.surface_of(input)
			})
			.xpect_eq(Some(host));
	}

	/// End-to-end proof the wiring survives a wrapper: with the composer nested
	/// under an extra `<div>`, real keystrokes still reach the focused `<input>`
	/// and Enter submits, advancing the user turn so the agent replies.
	#[beet_core::test]
	async fn keyboard_submit_drives_reply_through_wrapper() {
		let (mut app, host) = charcell_app().await;
		let thread = app
			.world_mut()
			.spawn((Thread::default(), Sequence::new(), children![
				(Actor::user(), UserInput),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();

		// mount the view directly but the composer under an extra wrapper <div>
		app.world_mut().entity_mut(host).insert((
			LayoutStyle::flex_col(),
			children![
				ThreadView::new(thread),
				(Element::new("div"), children![CreatePostForm::new(thread)]),
			],
		));
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		for _ in 0..25 {
			app.update();
		}

		app.world_mut()
			.get_mut::<ChannelTerminal>(host)
			.unwrap()
			.send_input(b"hello\r")
			.unwrap();
		for _ in 0..40 {
			app.update();
		}

		let frame = frame_plain(&app, host);
		// author label and body render on their own lines now, role-styled
		frame.as_str().xpect_contains("User");
		frame.as_str().xpect_contains("hello");
		frame.as_str().xpect_contains("Agent");
		frame.as_str().xpect_contains("you said: hello");
	}
}
