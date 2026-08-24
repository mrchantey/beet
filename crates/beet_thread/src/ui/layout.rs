//! The minimal document shell a thread scene's routes are wrapped in.
//!
//! A thread app is a router app, so its pages are themed the same way a site's
//! are: the layout middleware wraps every route, [`PageClasses`] pins the
//! session scheme per request, and the web-only head chrome is gated on the
//! HTML target. What it deliberately lacks is chrome: no header, sidebar or
//! footer, just a full-height column for the transcript and its composer.

use beet_core::prelude::*;
use beet_router::prelude::*;
use beet_ui::prelude::material::colors;
use beet_ui::prelude::*;
// the head-chrome widget, imported by name so the tag resolves regardless of
// which glob (`beet_router::prelude` vs `beet_ui::prelude`) also defines a `Reset`.
use beet_ui::prelude::Reset;

/// Wraps every descendant route in the thread document shell: a themed
/// full-height column, and nothing else.
///
/// The markup-spreadable form of `BaseLayout<ThreadShell>`, spread on the router
/// like any other layout middleware:
///
/// ```bsx
/// <Router {ThreadLayout}>
///     <Route path="/" {FixedPage}>..</Route>
/// </Router>
/// ```
///
/// The wrapper exists because a generic `BaseLayout<C>` cannot be authored from
/// markup, and [`BsxLayout`] resolves only `.bsx` file templates, not rust ones.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(BaseLayout<ThreadShell>)]
pub struct ThreadLayout;

/// The document shell itself: an html document whose body is one full-height
/// flex column around the page.
///
/// Reads the request-scoped [`RequestContextStack`] and the session [`Theme`] as
/// [`SiteLayout`] does, so `--color-scheme` reaches a thread page over the same
/// two hops a site page uses. The web `<head>` chrome (the CSS bake, the reset,
/// the color-scheme script) is non-visual in the terminal, where `<head>` is
/// `display: none`, so it is emitted only for the HTML target.
///
/// `pub(crate)` because [`ThreadLayout`] is its whole public surface.
#[template(system)]
pub(crate) fn ThreadShell(
	stack: Res<RequestContextStack>,
	theme: Res<Theme>,
) -> impl Bundle {
	let cx = stack.current();
	let body_classes = PageClasses::resolve(cx.parts(), &theme);
	let html_head = cx.parts().accepts(MediaType::Html).then(|| {
		rsx! {
			<Preflight/>
			<Reset/>
			<Stylesheet/>
			<ColorSchemeScript/>
		}
	});
	rsx! {
		<html lang="en">
			<head>{html_head}</head>
			<body {(body_classes, page_column())}>
				<Slot/>
			</body>
		</html>
	}
}

/// The page body's column: a viewport-height flex column tinted with the surface
/// palette, so the transcript grows and a composer pins to the bottom.
///
/// The shipped `.page` rule expresses the same column, but only once a rule set
/// is registered ([`MaterialStylePlugin`]); declaring it inline keeps a bare
/// thread app laid out correctly either way. Cascade styling (`inline_class!`),
/// since the thread UI's rows are cascade-styled and `resolve_styles` rebuilds
/// every node's `LayoutStyle` from the cascade, which would clobber a set
/// component.
fn page_column() -> impl Bundle {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Vertical
		),
		// stretch children across the full width, so the transcript and the
		// composer (and its top-border separator) span the terminal
		(
			style::common_props::AlignItemsProp,
			style::AlignItems::Stretch
		),
		(
			style::common_props::Height,
			style::Length::ViewportHeight(100.)
		),
		Declaration::token(
			style::common_props::BackgroundColor,
			colors::Surface
		),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::OnSurface
		),
	]
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_router::prelude::*;
	use beet_ui::prelude::*;
	use bevy::math::UVec2;

	/// The live-TUI stack a thread scene is served through, the in-process twin of
	/// `beet --main=examples/thread/chat.bsx`: the charcell render + input
	/// pipeline, live navigation, and the thread plugins.
	fn thread_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			NavigatorPlugin,
			LivePagePlugin,
			CharcellTuiPlugin,
		))
		.init_plugin::<ThreadPlugin>()
		.init_plugin::<ThreadUiPlugin>();
		app
	}

	/// A router serving `page` as its one persistent route, wrapped in
	/// [`ThreadLayout`] — the shape every thread example declares. `page` is the
	/// route's `children!` bundle, so its members are the page's own top-level
	/// nodes (a view and a composer, as siblings), never nested under a wrapper.
	/// Returns the router entity, which surfaces browse.
	fn spawn_router(app: &mut App, page: impl Bundle) -> Entity {
		app.world_mut()
			.spawn((Router, ThreadLayout, children![(
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
		let url = beet_net::prelude::Url::parse(url);
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
	async fn renders_window_posts() {
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

	/// Push an error post (5xx intent) authored by the thread's agent into its
	/// window, so the transcript snapshot exercises the `error` role styling
	/// without a failing network call.
	fn push_error_post(app: &mut App, thread: Entity) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let agent_id = agent_id(app, thread);
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

	/// The id of the thread's agent actor, the author of pushed posts.
	fn agent_id(app: &App, thread: Entity) -> ActorId {
		app.world()
			.get::<ThreadWindow>(thread)
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
		ThreadWindow::reduce_now(app.world_mut());
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
		ThreadWindow::reduce_now(app.world_mut());
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
		ThreadWindow::reduce_now(app.world_mut());
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
	/// `{FocusOnAdd}` focuses it — so typed bytes route to the right widget with
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

	/// A thread whose user turn precedes the agent's, so one `Sequence` call
	/// exercises both.
	fn user_then_agent() -> impl Bundle {
		(Thread::default(), Sequence::new(), children![
			(Actor::user(), UserInput),
			(Actor::agent(), MockPostStreamer::default()),
		])
	}

	/// The user's turn is a Sequence action: calling the thread reaches the
	/// `User` actor's [`UserInput`], which waits for the composer's [`Submit`],
	/// appends the typed post, then passes so the agent replies to it. The
	/// `Submit` is fired directly here (the focus/typing path is `beet_ui`'s); a
	/// full keystroke run is `keyboard_submit_drives_reply`.
	#[beet_core::test]
	async fn user_input_advances_on_submit() {
		let (mut app, surface, thread) =
			serve_thread(UVec2::new(40, 12), user_then_agent()).await;
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		// the composer renders, then the user turn installs its Submit observer
		drive_until(&mut app, surface, "Send");
		for _ in 0..25 {
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

		drive_until(&mut app, surface, "you said: hello")
			.xpect_contains("User")
			.xpect_contains("Agent");
	}

	/// The full deterministic interaction: real keystrokes through the terminal
	/// input bridge type into the focused form and Enter submits, advancing the
	/// user turn so the mock agent replies. The bytes arrive on the surface's own
	/// channel, exactly as a `StdioTerminal` or an SSH session delivers them.
	#[beet_core::test]
	async fn keyboard_submit_drives_reply() {
		let (mut app, surface, thread) =
			serve_thread(UVec2::new(40, 12), user_then_agent()).await;
		app.world_mut()
			.entity_mut(thread)
			.insert(CallOnSpawn::<(), Outcome>::new(()));
		// the composer builds + focuses, then the user turn installs its observer
		drive_until(&mut app, surface, "Send");
		for _ in 0..25 {
			app.update();
		}

		app.world_mut()
			.get_mut::<ChannelTerminal>(surface)
			.unwrap()
			.send_input(b"hello\r")
			.unwrap();

		drive_until(&mut app, surface, "you said: hello")
			.xpect_contains("User")
			.xpect_contains("Agent");
	}
}
