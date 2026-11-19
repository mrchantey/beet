//! The reactive transcript widget: a [`ThreadWindow`] projected into a
//! [`Document`] and rendered as a scrollable, keyed list of posts.

use crate::prelude::*;
use beet_action::prelude::Outcome;
use beet_core::prelude::*;
// `ScrollPosition` is beet_ui's renderer-agnostic type; pin it explicitly so it wins
// over bevy's same-named ui type (reached via `beet_core::prelude` under
// `bevy_default`). The rest of beet_ui's prelude arrives via `crate::prelude`.
use beet_ui::prelude::RenderTreeQuery;
use beet_ui::prelude::ScrollPosition;
// styling for the transcript: an `inline_class!` per role keeps the rule colocated
// with the widget (a callsite-keyed class, so each role's literal is its own rule).
use beet_ui::prelude::Declaration;
use beet_ui::prelude::inline_class;
use beet_ui::prelude::material::colors;
use beet_ui::prelude::style;

// ═══════════════════════════════════════════════════════════════════════
// ThreadView: the reactive chat widget
// ═══════════════════════════════════════════════════════════════════════

/// A reactive view of a thread, rendering its [`ThreadWindow`] as a scrollable
/// list of posts. Carries its own [`Document`] (seeded empty, filled by
/// [`project_window_to_document`]), so the inner [`ReactiveChildren`] and the
/// per-row [`FieldRef`]s resolve against it via `DocumentPath::Ancestor`.
///
/// Host-agnostic content, not a host: spawn it under any render surface (a
/// charcell terminal host, a web page) and bind it to its thread with an
/// [`OfThread`] relationship. A marker, so the bound thread lives in the
/// relationship, not a stored field. From markup the view is the tag and its
/// binding a spread on the same entity, so the same view serves a local terminal
/// and a per-connection server surface alike:
///
/// ```rsx
/// <Thread bx:ref="thread" {Sequence}>..</Thread>
/// <ThreadView {OfThread($thread)}/>
/// ```
///
/// A window is bounded ([`MountThreadStore::tail`]), so a reader scrolling to
/// the top of the transcript pages older posts from the thread's
/// [`ThreadStore`] into the view's [`ThreadHistory`], view state that never
/// enters the window and so never the model's context. Session brackets
/// render as dividers.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Document, ThreadHistory)]
#[component(on_add = thread_view_on_add)]
pub struct ThreadView;

/// The older posts a [`ThreadView`] paged from its thread's store, oldest
/// first, view state the window never holds. Rebinding the view to another
/// thread clears it.
#[derive(Debug, Default, Clone, Component)]
pub struct ThreadHistory {
	posts: Vec<Post>,
	/// Whether the store holds nothing older.
	exhausted: bool,
	/// Whether a page is being read.
	loading: bool,
}

impl ThreadHistory {
	/// The paged posts, oldest first.
	pub fn posts(&self) -> &[Post] { &self.posts }
	/// Whether the store holds nothing older than these.
	pub fn exhausted(&self) -> bool { self.exhausted }
}

/// Ask a [`ThreadView`] to page the posts before the oldest it shows from
/// its thread's store, triggered as its reader scrolls to the top.
#[derive(Debug, Clone, Copy, EntityEvent)]
pub struct LoadOlderPosts {
	/// The view.
	pub entity: Entity,
}

impl LoadOlderPosts {
	/// How many posts one page reads.
	pub const PAGE: usize = 50;
}

impl ThreadView {
	/// A view bound to `thread`. Its reactive content is attached in `on_add`, so
	/// the bundle works both as a direct spawn and as a markup spread.
	pub fn new(thread: Entity) -> impl Bundle { (Self, OfThread(thread)) }
}

/// Attach the reactive post list when a [`ThreadView`] is added: a scroll
/// container whose children are one row per `posts` item, keyed by post id so
/// appends reuse settled rows.
fn thread_view_on_add(mut world: DeferredWorld, cx: HookContext) {
	// `insert(rsx!{<div ..>})` merges the element onto the view entity, so the view
	// *is* the scrolling transcript container. All styling goes through the cascade
	// (`inline_class!`): `resolve_styles` rebuilds `LayoutStyle`/`BoxStyle` from the
	// cascade for every node of a styled tree, so a set component would be clobbered.
	// Rows are a keyed `ReactiveChildren` over the document's `posts`.
	world.commands().entity(cx.entity).insert(rsx! {
		<div {(
			ThreadScroll::default(),
			ScrollPosition::default(),
			FieldRef::new("posts"),
			ReactiveChildren::keyed(post_key, post_row),
			transcript_style(),
		)}/>
	});
}

/// The scrolling transcript: a padded column that grows to fill its host, scrolls
/// on overflow, and spaces messages apart.
fn transcript_style() -> OnSpawn {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Vertical
		),
		(style::common_props::FlexGrowProp, 1_f32),
		(style::common_props::OverflowYProp, style::Overflow::Auto),
		(style::common_props::RowGapProp, style::Length::Rem(1.)),
		(
			style::common_props::Padding,
			style::Spacing::all(style::Length::Rem(1.))
		),
	]
}

/// Marks a [`ThreadView`]'s transcript container, and carries the follow state
/// [`follow_thread_scroll`] uses to keep the latest post in view.
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ThreadScroll {
	/// Whether the transcript tails the latest post. On by default, off while
	/// the reader has scrolled up, back on when they return to the bottom.
	following: bool,
	/// Frames of pinning left to apply, counted down as the follow re-asserts
	/// itself over the frames the new rows take to lay out.
	pending_follow: u8,
	/// Whether the reader was at the top of an overflowing transcript last
	/// frame, so arriving there pages older posts once.
	at_top: bool,
	/// The scrollable extent last seen, so growing content is not mistaken for
	/// the reader scrolling.
	last_max: i32,
}

impl Default for ThreadScroll {
	fn default() -> Self {
		Self {
			following: true,
			pending_follow: 0,
			at_top: false,
			last_max: 0,
		}
	}
}

/// Stable reconciliation key for a post item: its `id` field (a uuid string), so
/// reconciliation reuses a row across appends and in-progress body growth.
fn post_key(item: &Value) -> String {
	item.get("id")
		.and_then(|id| id.as_str().ok())
		.unwrap_or_default()
		.to_string()
}

/// Build one post row: a role-accented block holding the author label and the
/// post body bound through a [`FieldRef`] so streamed text re-syncs in place. The
/// row's terminating scope is `posts[index]`, so `text` resolves to
/// `posts[index].text`. Styling is keyed on the projected `kind`
/// (`system`/`user`/`agent`/`error`) so each speaker reads distinctly.
fn post_row(_index: usize, item: &Value) -> OnSpawn {
	let field = |name: &str| {
		item.get(name)
			.and_then(|value| value.as_str().ok())
			.unwrap_or_default()
			.to_string()
	};
	let author = field("author");
	let text = field("text");
	let kind = item
		.get("kind")
		.and_then(|kind| kind.as_str().ok())
		.unwrap_or("agent")
		.to_string();
	if kind == "session" {
		return OnSpawn::insert(rsx! {
			<div {divider_style()}>{text}</div>
		});
	}
	OnSpawn::insert(rsx! {
		<div {message_block(&kind)}>
			<div {author_label(&kind)}>{author}</div>
			// seeded with the body at build time so a new row paints its text on
			// the first frame, bound so streamed growth re-syncs in place
			<div {body_text()}>{(Value::new(text), FieldRef::new("text"))}</div>
		</div>
	})
}

/// A session bracket: a dim, centered rule between sessions.
fn divider_style() -> OnSpawn {
	inline_class![
		(style::common_props::TextAlignProp, style::TextAlign::Center),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::Outline
		),
	]
}

/// The block wrapping one message: a role-tinted left accent bar with the body
/// padded off it (inter-message spacing is the scroll container's `row-gap`).
/// Each arm is a distinct `inline_class!` callsite so the roles resolve to
/// distinct (callsite-keyed) rules rather than colliding on one.
fn message_block(kind: &str) -> OnSpawn {
	match kind {
		"user" => inline_class![
			(style::common_props::BorderLeftWidth, style::Length::Rem(1.)),
			Declaration::token(
				style::common_props::BorderColorProp,
				colors::Primary
			),
			(style::common_props::Padding, block_padding()),
		],
		"error" => inline_class![
			(style::common_props::BorderLeftWidth, style::Length::Rem(1.)),
			Declaration::token(
				style::common_props::BorderColorProp,
				colors::Error
			),
			(style::common_props::Padding, block_padding()),
		],
		"system" | "developer" => inline_class![
			(style::common_props::BorderLeftWidth, style::Length::Rem(1.)),
			Declaration::token(
				style::common_props::BorderColorProp,
				colors::Outline
			),
			(style::common_props::Padding, block_padding()),
		],
		// agent and any unknown role
		_ => inline_class![
			(style::common_props::BorderLeftWidth, style::Length::Rem(1.)),
			Declaration::token(
				style::common_props::BorderColorProp,
				colors::Tertiary
			),
			(style::common_props::Padding, block_padding()),
		],
	}
}

/// Left padding holding a message's text off its accent bar. A plain value reused
/// by every [`message_block`] arm (not an `inline_class!` call, so no callsite
/// collision).
fn block_padding() -> style::Spacing {
	style::Spacing {
		left: style::Length::Rem(1.),
		..Default::default()
	}
}

/// The author label line: bold and tinted by role.
fn author_label(kind: &str) -> OnSpawn {
	match kind {
		"user" => inline_class![
			(style::common_props::FontWeightProp, style::FontWeight::Bold),
			Declaration::token(
				style::common_props::ForegroundColor,
				colors::Primary
			),
		],
		"error" => inline_class![
			(style::common_props::FontWeightProp, style::FontWeight::Bold),
			Declaration::token(
				style::common_props::ForegroundColor,
				colors::Error
			),
		],
		"system" | "developer" => inline_class![
			(style::common_props::FontWeightProp, style::FontWeight::Bold),
			Declaration::token(
				style::common_props::ForegroundColor,
				colors::Outline
			),
		],
		_ => inline_class![
			(style::common_props::FontWeightProp, style::FontWeight::Bold),
			Declaration::token(
				style::common_props::ForegroundColor,
				colors::Tertiary
			),
		],
	}
}

/// The message body: wraps long lines and preserves authored newlines.
fn body_text() -> impl Bundle {
	inline_class![
		(
			style::common_props::WhiteSpaceProp,
			style::WhiteSpace::PreWrap
		),
		(
			style::common_props::WordBreakProp,
			style::WordBreak::BreakWord
		),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::OnSurface
		),
	]
}

// ═══════════════════════════════════════════════════════════════════════
// Projection: ThreadWindow -> Document
// ═══════════════════════════════════════════════════════════════════════

/// Project every changed [`ThreadWindow`] into the [`Document`] of each
/// [`ThreadView`] bound to its thread, and (per the contract) into a
/// [`Document`] on the thread entity itself.
///
/// The document holds every post (display intent or not, so reasoning and tool
/// traffic remain inspectable) as `{ "posts": [{ id, author, text }, ..] }`,
/// a view's paged [`ThreadHistory`] before its window. A view re-projects when
/// its thread's window changes, its history grows, or it is bound to another
/// thread, which clears its history. Keyed reconciliation downstream means a
/// grown in-progress body updates a row rather than rebuilding it, so
/// streaming flows through the bound [`Value`].
pub(crate) fn project_window_to_document(
	mut commands: Commands,
	windows: Query<(Entity, Ref<ThreadWindow>)>,
	mut views: Query<
		(Entity, Ref<OfThread>, &mut ThreadHistory),
		With<ThreadView>,
	>,
	mut documents: Query<&mut Document>,
) -> Result {
	// the contract's thread-side document, inserted if absent
	for (thread_entity, window) in
		windows.iter().filter(|(_, window)| window.is_changed())
	{
		set_document(
			&mut commands,
			&mut documents,
			thread_entity,
			project_window(&window, &[]),
		);
	}
	for (view, of_thread, mut history) in views.iter_mut() {
		if of_thread.is_changed() && !of_thread.is_added() {
			*history = default();
		}
		let Ok((_, window)) = windows.get(of_thread.thread()) else {
			continue;
		};
		if !(window.is_changed()
			|| of_thread.is_changed()
			|| history.is_changed())
		{
			continue;
		}
		set_document(
			&mut commands,
			&mut documents,
			view,
			project_window(&window, history.posts()),
		);
	}
	Ok(())
}

/// Page older posts when the reader arrives at the top of an overflowing
/// transcript, once per arrival.
pub(crate) fn page_on_scroll_top(
	mut commands: Commands,
	views: Query<(Entity, &ThreadHistory), With<ThreadView>>,
	children: Query<&Children>,
	tree: RenderTreeQuery,
	mut transcripts: Query<&mut ThreadScroll>,
	scrolls: Query<&ScrollPosition>,
) {
	for (view, history) in views.iter() {
		let Some(transcript) = std::iter::once(view)
			.chain(children.iter_descendants(view))
			.find(|entity| transcripts.contains(*entity))
		else {
			continue;
		};
		let at_top = tree
			.iter_ancestors_inclusive(transcript)
			.filter_map(|entity| scrolls.get(entity).ok())
			.find(|scroll| scroll.max.y > 0)
			.is_some_and(|scroll| scroll.offset.y <= 0);
		let Ok(mut scroll) = transcripts.get_mut(transcript) else {
			continue;
		};
		if at_top && !scroll.at_top && !history.exhausted && !history.loading {
			commands.trigger(LoadOlderPosts { entity: view });
		}
		scroll.at_top = at_top;
	}
}

/// Observer: read the page before the oldest post a view shows from its
/// thread's store into its [`ThreadHistory`]. The cursor is the oldest post
/// of the window's tail rather than a current seed loaded however old, and
/// a page holding posts the window already shows has them dropped at
/// projection.
pub(crate) fn load_older_posts(
	ev: On<LoadOlderPosts>,
	async_commands: AsyncCommands,
	mut views: Query<(&OfThread, &mut ThreadHistory)>,
	threads: Query<(&Thread, &ThreadWindow, &ThreadStore)>,
) -> Result {
	let view = ev.entity;
	let (of_thread, mut history) = views.get_mut(view)?;
	let Ok((thread, window, store)) = threads.get(of_thread.thread()) else {
		return Ok(());
	};
	let Some(cursor) = history
		.posts
		.first()
		.or_else(|| {
			window
				.posts()
				.iter()
				.find(|post| !window.seed().contains(&post.id()))
		})
		.map(Post::id)
	else {
		history.exhausted = true;
		return Ok(());
	};
	history.loading = true;
	let (thread, store) = (thread.clone(), store.clone());
	async_commands.run(async move |world: AsyncWorld| -> Result {
		let page = store
			.page_before(&thread, cursor, LoadOlderPosts::PAGE)
			.await?;
		world
			.entity(view)
			.with(move |mut entity: EntityWorldMut| {
				if let Some(mut history) = entity.get_mut::<ThreadHistory>() {
					history.exhausted = page.len() < LoadOlderPosts::PAGE;
					history.loading = false;
					let older = std::mem::take(&mut history.posts);
					history.posts = page.into_iter().chain(older).collect();
				}
			})
			.await
	});
	Ok(())
}

/// Follow-on-append: when a [`ThreadView`]'s document changes (a post was
/// added or grew) *or* its keyed rows spawn in, arm a follow, then for the next
/// few frames pin every scroll container showing the transcript to the bottom
/// by parking its offset past the end. `clamp_scroll_positions` re-clamps that
/// to the true max against the laid-out content.
///
/// The follow spans frames rather than landing in one because the geometry
/// trails the content: rows spawn a frame after the document is set, then lay
/// out and clamp, so a container's extent only reflects them afterwards. It can
/// even change hands in that window, since a transcript that outgrows its host
/// takes over the scrolling from the host's scrollport. Pinning the whole path
/// for a few frames follows the tail whichever container ends up owning it, and
/// each container simply clamps to its own bottom.
///
/// A reader who has scrolled up is left there. Their intent is read only on
/// *settled* frames (no new content, no moving extent), where the scroll
/// position is theirs alone: at the bottom keeps the tail on, anywhere else
/// turns it off until they scroll back. Sampling it while content lands would
/// instead read a transcript that has simply outgrown the viewport as a reader
/// who scrolled away, and stop following on the first burst.
pub(crate) fn follow_thread_scroll(
	views: Query<
		(Entity, Ref<Document>, Option<Ref<Children>>),
		With<ThreadView>,
	>,
	children: Query<&Children>,
	// the transcript and every container it renders inside, innermost first:
	// ancestry is visual, so a router page reaches the surface scrollport
	// hosting it across its transclusion, the walk the charcell wheel/key
	// scrolling uses to resolve its container.
	tree: RenderTreeQuery,
	mut transcripts: Query<&mut ThreadScroll>,
	mut scrolls: Query<&mut ScrollPosition>,
) {
	for (view, document, rows) in views.iter() {
		// the transcript container is the view entity itself (the `<div>` merges
		// onto it on insert) or a descendant carrying `ThreadScroll`.
		let Some(transcript) = std::iter::once(view)
			.chain(children.iter_descendants(view))
			.find(|entity| transcripts.contains(*entity))
		else {
			continue;
		};
		let path = tree
			.iter_ancestors_inclusive(transcript)
			.collect::<Vec<_>>();
		// the innermost container with somewhere to scroll owns the reader's
		// position; with nothing scrollable yet there is nothing to follow.
		let scroller = path
			.iter()
			.filter_map(|entity| scrolls.get(*entity).ok())
			.find(|scroll| scroll.max.y > 0);
		let max = scroller.map(|scroll| scroll.max.y).unwrap_or_default();
		let at_bottom = scroller.is_none_or(|scroll| scroll.at_bottom());
		let new_content =
			document.is_changed() || rows.is_some_and(|rows| rows.is_changed());
		let Ok(mut thread_scroll) = transcripts.get_mut(transcript) else {
			continue;
		};
		// with the geometry stable and no new content, wherever the reader has
		// left the scroll *is* their intent: at the bottom means keep tailing.
		if !new_content && max == thread_scroll.last_max {
			thread_scroll.following = at_bottom;
		}
		// new posts, or rows still laying out (the extent is still moving, and
		// which container scrolls can change hands as the transcript outgrows
		// its host), so re-assert the pin
		if thread_scroll.following
			&& (new_content || max != thread_scroll.last_max)
		{
			thread_scroll.pending_follow = FOLLOW_FRAMES;
		}
		thread_scroll.last_max = max;
		if thread_scroll.pending_follow == 0 {
			continue;
		}
		thread_scroll.pending_follow -= 1;
		// `clamp_scroll_positions` settles each of these to its own bottom
		for entity in path {
			if let Ok(mut scroll) = scrolls.get_mut(entity) {
				scroll.offset.y = ScrollPosition::END;
			}
		}
	}
}

/// How many frames a follow keeps pinning: enough to outlast the row spawn, the
/// layout pass and the clamp that gives the containers their extent.
const FOLLOW_FRAMES: u8 = 4;

/// Build the document value for a window and the older posts paged before
/// it: a `posts` list of `{ id, author, kind, text }`, each paged post the
/// window also holds shown once, as the window's.
fn project_window(window: &ThreadWindow, older: &[Post]) -> Value {
	let shown = window.posts().iter().map(Post::id).collect::<HashSet<_>>();
	let older = older
		.iter()
		.filter(|post| !shown.contains(&post.id()))
		.filter_map(|post| {
			window
				.actor(post.author())
				.ok()
				.map(|actor| AuthoredPost { post, actor })
		});
	let posts = older
		.chain(window.authored_posts())
		.map(|view| {
			Value::Map(
				[
					("id".into(), Value::new(view.post.id().to_string())),
					("author".into(), Value::new(view.actor.name())),
					("kind".into(), Value::new(post_kind(&view))),
					("text".into(), Value::new(post_text(&view))),
				]
				.into_iter()
				.collect(),
			)
		})
		.collect::<Vec<_>>();
	Value::Map([("posts".into(), Value::List(posts))].into_iter().collect())
}

/// The text a row shows: a session bracket as a divider line, any other post
/// as its body.
fn post_text(view: &AuthoredPost) -> String {
	if let Some(opened) = SessionOpened::of(view.post) {
		return match opened.spawned_by {
			Some(cause) => format!("── session opened by {cause} ──"),
			None => "── session opened ──".to_string(),
		};
	}
	if let Some(closed) = SessionClosed::of(view.post) {
		let outcome = match closed.outcome {
			Outcome::Pass(_) => "pass",
			Outcome::Fail(_) => "fail",
		};
		return format!(
			"── session closed: {outcome} in {}s ──",
			closed.duration.as_secs()
		);
	}
	view.post.to_string()
}

/// The display role of a post, used by [`post_row`] for styling: a session
/// bracket renders as `session`, a 5xx error post as `error`, otherwise it
/// follows the authoring actor's kind.
fn post_kind(view: &AuthoredPost) -> &'static str {
	if view.post.intent().is_session() {
		return "session";
	}
	if view.post.intent().is_server_error() {
		return "error";
	}
	match view.actor.kind() {
		ActorKind::System => "system",
		ActorKind::Developer => "developer",
		ActorKind::User => "user",
		ActorKind::Agent => "agent",
	}
}

/// Update `entity`'s [`Document`] in place, or insert one if it has none yet.
fn set_document(
	commands: &mut Commands,
	documents: &mut Query<&mut Document>,
	entity: Entity,
	value: Value,
) {
	match documents.get_mut(entity) {
		Ok(mut document) => document.0 = value,
		Err(_) => {
			commands.entity(entity).insert(Document::new(value));
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::ui::test_ext;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	/// The live tail: posts appended *after* the first projection must reach the
	/// view too, both as document entries and as spawned rows. Regression for a
	/// transcript that renders its first batch and then freezes.
	#[beet_core::test]
	fn projects_appended_posts() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).install_type::<(
			DocumentPlugin,
			ThreadPlugin,
			ThreadUiPlugin,
		)>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadWindow::default()))
			.id();
		let view = app.world_mut().spawn(ThreadView::new(thread)).id();
		app.update();

		// the projection runs in `Update` and the document chain in the next
		// frame's `PreUpdate`, so a row lands one frame after its post
		push_post(&mut app, thread, "first");
		app.update();
		app.update();
		row_count(&app, view).xpect_eq(1);

		push_post(&mut app, thread, "second");
		app.update();
		app.update();
		row_count(&app, view).xpect_eq(2);
	}

	/// The same live tail, but with the view built through the template
	/// substrate (`spawn_template`), the path a router page takes.
	#[beet_core::test]
	fn projects_appended_posts_through_template() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).install_type::<(
			DocumentPlugin,
			ThreadPlugin,
			ThreadUiPlugin,
		)>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadWindow::default()))
			.id();
		let page = app
			.world_mut()
			.spawn_template(Snippet::from_bundle(rsx! {
				<div><ThreadView {OfThread(thread)}/></div>
			}))
			.unwrap()
			.id();
		app.update();
		let view = app
			.world_mut()
			.query_filtered::<Entity, With<ThreadView>>()
			.single(app.world())
			.unwrap();
		app.world()
			.entity(view)
			.get::<OfThread>()
			.unwrap()
			.thread()
			.xpect_eq(thread);
		let _ = page;

		push_post(&mut app, thread, "first");
		app.update();
		app.update();
		row_count(&app, view).xpect_eq(1);

		push_post(&mut app, thread, "second");
		app.update();
		app.update();
		row_count(&app, view).xpect_eq(2);
	}

	/// The painted surface follows the tail: a post appended after the first
	/// frame is repainted, not just projected.
	#[beet_core::test]
	fn repaints_appended_posts() {
		let (mut app, thread, host) = live_host();
		push_post(&mut app, thread, "first post");
		settle(&mut app);
		frame(&app, host).xpect_contains("first post");

		push_post(&mut app, thread, "second post");
		settle(&mut app);
		frame(&app, host).xpect_contains("second post");
	}

	/// The tail stays visible when the transcript overflows its host, including
	/// when the very first projection already overflows (a feed's opening page,
	/// rather than a chat's one-post-at-a-time growth).
	#[beet_core::test]
	fn follows_the_tail_when_overflowing() {
		let (mut app, thread, host) = live_host();
		// a burst that overflows the 12 row viewport in one projection
		(0..20).for_each(|index| {
			push_post(&mut app, thread, &format!("post {index}"))
		});
		settle(&mut app);
		frame(&app, host).xpect_contains("post 19");

		// and it keeps following as later posts arrive
		push_post(&mut app, thread, "newest post");
		settle(&mut app);
		frame(&app, host).xpect_contains("newest post");
	}

	/// A page-bound view of a fresh thread in the live-TUI render stack: the
	/// app, the thread entity and the host whose buffer is painted.
	fn live_host() -> (App, Entity, Entity) {
		let mut app = test_ext::render_app();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadWindow::default()))
			.id();
		let (host, _) = test_ext::mount_page(
			&mut app,
			UVec2::new(40, 12),
			Snippet::from_bundle(rsx! { <ThreadView {OfThread(thread)}/> }),
		);
		(app, thread, host)
	}

	fn settle(app: &mut App) { test_ext::settle(app); }
	fn frame(app: &App, host: Entity) -> String { test_ext::frame(app, host) }

	/// Append one completed text post to the thread's window.
	fn push_post(app: &mut App, thread: Entity, text: &str) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let mut window =
			app.world_mut().get_mut::<ThreadWindow>(thread).unwrap();
		let actor = window.insert_actor(Actor::agent());
		window.upsert_post(AgentPost::new_text(
			actor,
			thread_id,
			text,
			PostStatus::Completed,
		));
	}

	/// The reactive rows the view has spawned for its posts.
	fn row_count(app: &App, view: Entity) -> usize {
		app.world()
			.entity(view)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.filter(|child| {
						app.world().get::<ReactiveChild>(*child).is_some()
					})
					.count()
			})
			.unwrap_or_default()
	}

	/// The declarative binding the scenes rely on: an `{(ThreadView, OfThread($thread))}`
	/// spread resolves its `$thread` reference to the `bx:ref="thread"` Thread entity
	/// (the relationship machinery remaps it), so the view renders the thread a sibling
	/// subtree declares, with no Rust glue. The thread then names the view through its
	/// `ThreadItems`.
	#[beet_core::test]
	fn thread_view_binds_to_referenced_thread() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<(ThreadPlugin, ThreadUiPlugin)>();
		let source = r#"
<Fragment>
	<Thread bx:ref="thread" {Sequence}/>
	<ThreadView {OfThread($thread)}/>
</Fragment>
"#;
		BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.world_mut().flush();

		let thread = app
			.world_mut()
			.query_filtered::<Entity, With<Thread>>()
			.single(app.world())
			.unwrap();
		let (view, of_thread) = app
			.world_mut()
			.query_filtered::<(Entity, &OfThread), With<ThreadView>>()
			.single(app.world())
			.unwrap();
		// the `$thread` placeholder remapped to the real Thread entity ...
		of_thread.thread().xpect_eq(thread);
		// ... and the thread names the view back through its `ThreadItems`.
		app.world()
			.entity(thread)
			.get::<ThreadItems>()
			.unwrap()
			.iter()
			.any(|item| item == view)
			.xpect_true();
	}

	/// A thread of more posts than its window's bound shows its tail, and a
	/// view pages the rest from the store, never into the window.
	#[beet_core::test]
	async fn pages_older_posts_from_the_store() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).install_type::<(
			DocumentPlugin,
			ThreadPlugin,
			ThreadUiPlugin,
		)>();
		let store = ThreadStore::default();
		let record = Thread::default();
		let actor = Actor::agent();
		store.insert_thread(record.clone()).await.unwrap();
		store.insert_actor(actor.clone()).await.unwrap();
		store
			.insert_posts(
				(0..5)
					.map(|index| {
						AgentPost::new_text(
							actor.id(),
							record.id(),
							format!("post {index}"),
							PostStatus::Completed,
						)
					})
					.collect(),
			)
			.await
			.unwrap();
		let window = store.materialize_window(&record, Some(2)).await.unwrap();
		let synced = window.posts().iter().map(Post::id).collect();
		let thread = app
			.world_mut()
			.spawn((record, window, store, SyncedPosts::new(synced)))
			.id();
		let view = app.world_mut().spawn(ThreadView::new(thread)).id();
		app.update();
		app.update();
		row_count(&app, view).xpect_eq(2);

		app.world_mut().trigger(LoadOlderPosts { entity: view });
		for _ in 0..6 {
			app.update();
		}
		row_count(&app, view).xpect_eq(5);
		app.world()
			.get::<ThreadHistory>(view)
			.unwrap()
			.exhausted()
			.xpect_true();
		app.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.posts()
			.len()
			.xpect_eq(2);
	}
}
