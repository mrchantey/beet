//! The served chat surface, end to end: a thread scene's view and composer
//! wrapped in the shipped [`AppShell`], booted into the live-TUI stack and
//! driven with terminal bytes.
//!
//! What only this suite can show is that the pieces assemble: the projection
//! reaching a painted frame, the composer's keystrokes posting and so passing a
//! person's turn,
//! and the transcript scrolling inside its own region rather than shoving the
//! composer off screen.

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;
	use beet_ui::prelude::*;
	// explicit: bevy_ui ships its own `ScrollPosition`, which reaches this module
	// through `beet_core::prelude` whenever a co-compiled crate enables that
	// feature. The charcell view means beet_ui's.
	use beet_ui::prelude::ScrollPosition;
	use bevy::math::UVec2;

	/// The live-TUI stack a thread scene is served through, the in-process twin of
	/// `beet --entry=examples/thread/chat.bsx`: the charcell render + input
	/// pipeline, live navigation, and the thread plugins.
	fn thread_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			NavigatorPlugin,
			PageHostPlugin,
			CharcellTuiPlugin,
		))
		.install_types::<(ThreadPlugin, ThreadUiPlugin)>();
		app
	}

	/// A router serving `page` as its one persistent route, wrapped in the
	/// [`AppShell`], the shape every thread example declares. `page` is the
	/// route's `children!` bundle, so its members are the page's own top-level
	/// nodes (a view and a composer, as siblings), never nested under a wrapper.
	/// Returns the router entity, which surfaces browse.
	fn spawn_router(app: &mut App, page: impl Bundle) -> Entity {
		app.world_mut()
			.spawn((Router, Layout::of::<AppShell>(), children![(
				route::new("", FixedPage),
				page
			)]))
			.flush()
	}

	/// Open a surface onto `router`: a channel terminal paired with the page-host
	/// buffer and an in-world navigator opening on `/`, exactly what `TuiServer`
	/// spawns for the local terminal and `SshTuiServer` per connection.
	fn open_surface(app: &mut App, router: Entity, size: UVec2) -> Entity {
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		app.world_mut()
			.spawn((
				channel,
				terminal,
				PageHost::bundle(size),
				Navigator::in_world(router, ""),
			))
			.id()
	}

	/// An app + surface serving `page`, settled so the composer's store-backed
	/// template registration has landed. Returns `(app, surface)`.
	async fn serve(size: UVec2, page: impl Bundle) -> (App, Entity) {
		let mut app = thread_app();
		// settle Startup + the async `CreatePostForm` registration before any
		// content is attached, so a form in the page resolves deterministically.
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		let router = spawn_router(&mut app, page);
		let surface = open_surface(&mut app, router, size);
		(app, surface)
	}

	/// The on-screen frame as plain text (the front buffer, as the live host
	/// paints to the back buffer then swaps).
	fn frame(app: &mut App, surface: Entity) -> String {
		app.update();
		app.world()
			.get::<DoubleBuffer>(surface)
			.map(|buffer| buffer.front_buffer().render_plain())
			.unwrap_or_default()
	}

	/// Drive the app until `surface`'s frame contains `needle`, returning it.
	fn drive_until(app: &mut App, surface: Entity, needle: &str) -> String {
		for _ in 0..200 {
			let frame = frame(app, surface);
			if frame.contains(needle) {
				return frame;
			}
		}
		panic!(
			"thread surface frame never contained '{needle}':\n{}",
			frame(app, surface)
		);
	}

	/// Navigate `surface` to `url` and settle the spawned task, so the next frame
	/// renders the new page.
	///
	/// [`Navigator::navigate_to`] is async, and [`drive_until`] cannot be the wait:
	/// a re-navigation usually carries the same text, so its needle is already on
	/// screen and the drive returns before the navigation has landed. On the
	/// single-threaded task pool it happened to land inside the first frame; under
	/// `beet_core/bevy_multithreaded` the task is dispatched to a worker and lands
	/// several frames later, so the settle is what makes this deterministic.
	async fn navigate(app: &mut App, surface: Entity, url: &str) {
		let url = beet_core::prelude::Url::coerce(url);
		app.world_mut()
			.entity_mut(surface)
			.run_async_local(move |entity| Navigator::navigate_to(entity, url));
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
	}

	/// The first rendered element with `tag`, eg the composer's `<form>` or
	/// `<input>`.
	fn element_by_tag(app: &mut App, tag: &str) -> Option<Entity> {
		app.world_mut().with_state::<ElementQuery, _>(|elements| {
			elements
				.iter()
				.find(|view| view.tag() == tag)
				.map(|view| view.entity)
		})
	}

	/// Spawn a thread and serve its view + composer, the shape every interactive
	/// example declares. Returns `(app, surface, thread)`.
	async fn serve_thread(
		size: UVec2,
		thread: impl Bundle,
	) -> (App, Entity, Entity) {
		let mut app = thread_app();
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		let thread = app.world_mut().spawn(thread).flush();
		let router = spawn_router(&mut app, children![
			ThreadView::new(thread),
			CreatePostForm::new(thread),
		]);
		let surface = open_surface(&mut app, router, size);
		(app, surface, thread)
	}

	/// End to end through the served shape: calling the thread runs the agent's
	/// turn (its `Sequence` child), which projects into the view's document and
	/// renders as charcell text on the surface, the agent's streamed echo included.
	#[beet_core::test]
	async fn renders_record_posts() {
		let (mut app, surface, thread) = serve_thread(
			UVec2::new(40, 12),
			(Thread::default(), Sequence::new(), children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			]),
		)
		.await;
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));

		// both rows render: author label + body, with the agent's streamed echo
		// flowing through the per-row FieldRef binding
		drive_until(&mut app, surface, "you said: hello")
			.xpect_contains("User")
			.xpect_contains("hello")
			.xpect_contains("Agent");
	}

	/// The page is themed per request, not per host: a `?color-scheme=` on the
	/// navigated url pins the scheme class on the served `<body>`, the same two
	/// hops the site uses (`TuiServer` seeds `Theme::scheme` from
	/// `--color-scheme`, [`PageClasses`] pins it per request).
	#[beet_core::test]
	async fn page_pins_color_scheme() {
		// the session default (dark) with no override ...
		let (mut app, surface) =
			serve(UVec2::new(40, 8), children![rsx! { <p>"hi"</p> }]).await;
		drive_until(&mut app, surface, "hi");
		let scheme_class = |app: &mut App, name: &ClassName| {
			let body = element_by_tag(app, "body").unwrap();
			app.world()
				.get::<Classes>(body)
				.is_some_and(|classes| classes.contains_name(name))
		};
		scheme_class(&mut app, &classes::DARK_SCHEME).xpect_true();

		// ... and a seeded light theme themes the next navigation
		app.world_mut().resource_mut::<Theme>().scheme = ColorScheme::Light;
		navigate(&mut app, surface, "").await;
		drive_until(&mut app, surface, "hi");
		scheme_class(&mut app, &classes::LIGHT_SCHEME).xpect_true();
		// the re-navigation replaced the page rather than adding to it, so the
		// previous scheme is gone. This is what the settle in `navigate` buys:
		// without it the assertion reads the *first* render's dark page.
		scheme_class(&mut app, &classes::DARK_SCHEME).xpect_false();
	}

	/// Push an error post ([`PostIntent::Error`]) authored by the thread's agent into its
	/// records, so the transcript snapshot exercises the `error` role styling
	/// without a failing network call.
	fn push_error_post(app: &mut App, thread: Entity) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let agent_id = agent_id(app, thread);
		app.world_mut()
			.get_mut::<ThreadRecords>(thread)
			.unwrap()
			.upsert_post(AgentPost::new_error(
				agent_id,
				thread_id,
				"model request failed: 401 Unauthorized",
				PostStatus::Completed,
			));
	}

	/// The id of the thread's agent actor, the author of pushed posts.
	fn agent_id(app: &App, thread: Entity) -> ActorId {
		app.world()
			.get::<ThreadRecords>(thread)
			.unwrap()
			.actors()
			.values()
			.find(|actor| actor.kind() == ActorKind::Agent)
			.unwrap()
			.id()
	}

	/// Append numbered text posts (`line NN`) authored by the thread's agent over
	/// `lines`, so a transcript can be grown past its viewport.
	fn push_numbered_posts(
		app: &mut App,
		thread: Entity,
		lines: std::ops::RangeInclusive<usize>,
	) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let agent_id = agent_id(app, thread);
		for index in lines {
			app.world_mut()
				.get_mut::<ThreadRecords>(thread)
				.unwrap()
				.upsert_post(AgentPost::new_text(
					agent_id,
					thread_id,
					format!("line {index:02}"),
					PostStatus::Completed,
				));
		}
	}

	/// The full chat surface (scrollable transcript + composer) renders every
	/// role with its own styling. A static thread (seed posts, no streamers) plus
	/// a pushed error post keeps the snapshot deterministic and offline.
	#[beet_core::test]
	async fn chat_surface_snapshot() {
		let (mut app, surface, thread) = serve_thread(
			UVec2::new(56, 24),
			(Thread::default(), Sequence::new(), children![
				(Actor::new("System", ActorKind::System), children![
					Post::spawn("you are a friendly robot")
				],),
				(Actor::new("Billy", ActorKind::User), children![
					Post::spawn("hello there robot")
				],),
				(Actor::new("BeepBot", ActorKind::Agent), children![
					Post::spawn("Beep boop! Greetings, human.")
				],),
			]),
		)
		.await;
		ThreadRecords::reduce_now(app.world_mut());
		push_error_post(&mut app, thread);
		drive_until(&mut app, surface, "401 Unauthorized");
		frame(&mut app, surface).xpect_snapshot();
	}

	/// A transcript taller than its viewport scrolls *inside its own region*
	/// rather than pushing the composer off screen: the composer stays visible,
	/// and `follow_thread_scroll` pins the view to the latest post (the earliest
	/// are clipped out the top). Regression for charcell internal scroll, where a
	/// `flex-grow` + `overflow-y: auto` child used to grow to its full content
	/// height and shove later flex items past the screen.
	#[beet_core::test]
	async fn chat_scroll_keeps_composer_visible() {
		let (mut app, surface, thread) = serve_thread(
			UVec2::new(48, 16),
			(Thread::default(), Sequence::new(), children![
				(Actor::new("System", ActorKind::System), children![
					Post::spawn("be brief")
				],),
				(Actor::new("Agent", ActorKind::Agent),),
			]),
		)
		.await;
		ThreadRecords::reduce_now(app.world_mut());
		// far more posts than the 16-row viewport can hold
		push_numbered_posts(&mut app, thread, 1..=14);
		let frame = drive_until(&mut app, surface, "line 14");
		// the composer survived (the transcript clipped instead of pushing it off) ...
		frame
			.as_str()
			.xpect_contains("Send")
			// ... and the earliest post is scrolled out of the clipped region.
			.xnot()
			.xpect_contains("line 01");
		frame.xpect_snapshot();
	}

	/// Follow-to-bottom only sticks while the reader is at the bottom: once they
	/// scroll up, a new post does not yank them back down. Regression for
	/// `follow_thread_scroll` wrestling the scroll from the user.
	#[beet_core::test]
	async fn follow_leaves_scrolled_up_reader() {
		let (mut app, surface, thread) = serve_thread(
			UVec2::new(48, 16),
			(Thread::default(), Sequence::new(), children![
				(Actor::new("System", ActorKind::System), children![
					Post::spawn("be brief")
				],),
				(Actor::new("Agent", ActorKind::Agent),),
			]),
		)
		.await;
		ThreadRecords::reduce_now(app.world_mut());
		push_numbered_posts(&mut app, thread, 1..=12);
		drive_until(&mut app, surface, "line 12");
		// settle the follow + the scroll clamp against the laid-out geometry, so
		// the reader's scroll below is measured against a real `max`
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
			.get_mut::<ScrollPosition>(scroll)
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
		let pos = app.world().get::<ScrollPosition>(scroll).unwrap();
		(pos.offset.y < pos.max.y).xpect_true();
		// still showing the top, not pinned to the latest
		frame(&mut app, surface).as_str().xpect_contains("line 01");
	}

	/// The widget builds its `<form>` from its rust template on add, and the
	/// served page renders it: a `<form>` with the `message` `<input>` and the
	/// `Send` `<button>`.
	#[beet_core::test]
	async fn composer_renders_form() {
		let (mut app, surface, _) = serve_thread(
			UVec2::new(40, 12),
			(Thread::default(), Sequence::new()),
		)
		.await;
		drive_until(&mut app, surface, "Send");
		element_by_tag(&mut app, "form").is_some().xpect_true();
		element_by_tag(&mut app, "input").is_some().xpect_true();
		element_by_tag(&mut app, "button").is_some().xpect_true();
	}

	/// The composer's `<input>` resolves to the surface serving it, however deep
	/// the page sits (layout, portal transclusion, wrapper element), and
	/// `{FocusOnAdd}` focuses it, so typed bytes route to the right widget with
	/// no per-widget surface wiring.
	#[beet_core::test]
	async fn composer_focus_resolves_to_surface() {
		let (mut app, surface, _) = serve_thread(
			UVec2::new(40, 12),
			(Thread::default(), Sequence::new()),
		)
		.await;
		drive_until(&mut app, surface, "Send");

		let input = element_by_tag(&mut app, "input").unwrap();
		app.world().entity(input).contains::<Focus>().xpect_true();
		app.world_mut()
			.with_state::<SurfaceQuery, _>(|surfaces| {
				surfaces.surface_of(input)
			})
			.xpect_eq(Some(surface));
	}

	/// A thread whose person's turn precedes the agent's, so one `Sequence` call
	/// exercises both.
	fn user_then_agent() -> impl Bundle {
		(Thread::default(), Sequence::new(), children![
			(Actor::user(), UserInput),
			(Actor::agent(), MockPostStreamer::default()),
		])
	}

	/// Submit `text` on the composer's `<form>`.
	fn submit(app: &mut App, text: &str) {
		let form = element_by_tag(app, "form").unwrap();
		let values = Value::Map(
			[("message".into(), Value::new(text))].into_iter().collect(),
		);
		app.world_mut().trigger(Submit { form, values });
	}

	/// The thread's posts that are not logs, as `text` or `text (status)`
	/// when not completed.
	fn said(app: &App, thread: Entity) -> Vec<String> {
		app.world()
			.get::<ThreadRecords>(thread)
			.unwrap()
			.content_posts()
			.map(|view| match view.post.status() {
				PostStatus::Completed => view.post.to_string(),
				status => format!("{} ({status:?})", view.post),
			})
			.collect()
	}

	/// The person's turn is a wait on the records: calling the thread reaches
	/// the `User` actor's [`UserInput`], the composer's submit posts, and the
	/// post passes the turn so the agent replies to it. The `Submit` is fired
	/// directly here (the focus/typing path is `beet_ui`'s); a full keystroke
	/// run is `keyboard_submit_drives_reply`.
	#[beet_core::test]
	async fn user_input_advances_on_submit() {
		let (mut app, surface, thread) =
			serve_thread(UVec2::new(40, 12), user_then_agent()).await;
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		drive_until(&mut app, surface, "Send");
		submit(&mut app, "hello");
		drive_until(&mut app, surface, "you said: hello")
			.xpect_contains("User")
			.xpect_contains("Agent");
	}

	/// The full deterministic interaction: real keystrokes through the terminal
	/// input bridge type into the focused form and Enter submits, posting and
	/// so passing the person's turn, and the mock agent replies. The bytes arrive
	/// on the surface's own channel, exactly as a `StdioTerminal` or an SSH
	/// session delivers them.
	#[beet_core::test]
	async fn keyboard_submit_drives_reply() {
		let (mut app, surface, thread) =
			serve_thread(UVec2::new(40, 12), user_then_agent()).await;
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		drive_until(&mut app, surface, "Send");
		app.world_mut()
			.get_mut::<ChannelTerminal>(surface)
			.unwrap()
			.send_input(b"hello\r")
			.unwrap();
		drive_until(&mut app, surface, "you said: hello")
			.xpect_contains("User")
			.xpect_contains("Agent");
	}

	/// A group chat, with no agent and no behavior: its composer posts at
	/// once, an empty submit is ignored, and the posts persist.
	#[beet_core::test]
	async fn a_group_chat_persists_its_posts() {
		let (mut app, surface, thread) = serve_thread(
			UVec2::new(40, 12),
			(
				Thread::default(),
				InMemoryStore::new(),
				MountThreadStore {
					path: RelPath::new("group_chat"),
					new: false,
				},
				children![Actor::user()],
			),
		)
		.await;
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| -> Result {
				Thread::enter(
					world.clone(),
					thread,
					ThreadEntry::parse(
						Request::get("group_chat").parts().clone(),
					)?,
				)
				.await
			})
			.await
			.unwrap();
		drive_until(&mut app, surface, "Send");
		submit(&mut app, "  ");
		submit(&mut app, "hello all");
		drive_until(&mut app, surface, "hello all");
		said(&app, thread).xpect_eq(vec!["hello all".to_string()]);

		let id = app.world().get::<Thread>(thread).unwrap().id();
		let store = app.world().get::<ThreadStore>(thread).unwrap().clone();
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				ThreadStore::flush(&world.entity(thread)).await
			})
			.await
			.unwrap();
		store
			.posts(id)
			.await
			.unwrap()
			.into_iter()
			.filter(|post| post.intent() != PostIntent::Log)
			.map(|post| post.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["hello all".to_string()]);
	}

	/// A chat whose agent thinks until stopped: `Repeat[Sequence[person,
	/// agent]]`, the person having said `go`.
	fn thinking_chat() -> impl Bundle {
		(Thread::default(), Repeat::new(), children![(
			Sequence::new(),
			children![
				(Actor::user(), UserInput, children![Post::spawn("go")]),
				(
					Actor::agent(),
					MockPostStreamer::with_script([MockTurn::Hold(
						"thinking".into()
					)])
				),
			]
		)])
	}

	/// Stop the thinking chat with `stop`: the agent's post is closed as
	/// interrupted, the person's turn waits for their next post, and the agent
	/// answers that.
	async fn stops_the_thinking_chat(stop: impl FnOnce(&mut App, Entity)) {
		let (mut app, surface, thread) =
			serve_thread(UVec2::new(48, 16), thinking_chat()).await;
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		drive_until(&mut app, surface, "thinking");
		stop(&mut app, surface);
		for _ in 0..30 {
			app.update();
		}
		// the person's turn waits: nothing more is said
		said(&app, thread).xpect_eq(
			["go", "thinking (Interrupted)"].map(String::from).to_vec(),
		);
		submit(&mut app, "next");
		drive_until(&mut app, surface, "you said: next");
	}

	/// The composer's `Stop` button interrupts every actor of its thread.
	#[beet_core::test]
	async fn stop_button_interrupts_the_thread() {
		stops_the_thinking_chat(|app, _| {
			let stop =
				app.world_mut().with_state::<ElementQuery, _>(|elements| {
					elements
						.iter()
						.find(|view| {
							view.tag() == "button"
								&& view.attribute_string("type") == "button"
						})
						.map(|view| view.entity)
						.unwrap()
				});
			app.world_mut()
				.entity_mut(stop)
				.trigger(PointerUp::new(stop));
		})
		.await;
	}

	/// Escape in the composer's focused input interrupts every actor of its
	/// thread.
	#[beet_core::test]
	async fn escape_interrupts_the_thread() {
		stops_the_thinking_chat(|app, surface| {
			app.world_mut()
				.get_mut::<ChannelTerminal>(surface)
				.unwrap()
				.send_input(b"\x1b")
				.unwrap();
		})
		.await;
	}
}
