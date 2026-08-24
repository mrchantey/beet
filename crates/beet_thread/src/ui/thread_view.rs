//! The reactive transcript widget: a [`ThreadWindow`] projected into a
//! [`Document`] and rendered as a scrollable, keyed list of posts.

use crate::prelude::*;
use beet_core::prelude::*;
// `ScrollPosition` is beet_ui's renderer-agnostic type; pin it explicitly so it wins
// over bevy's same-named ui type (reached via `beet_core::prelude` under
// `bevy_default`). The rest of beet_ui's prelude arrives via `crate::prelude`.
use beet_ui::prelude::PortalOf;
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
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Document)]
#[component(on_add = thread_view_on_add)]
pub struct ThreadView;

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
		(style::common_props::FlexGrowProp, 1u32),
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
	/// The scrollable extent last seen, so growing content is not mistaken for
	/// the reader scrolling.
	last_max: i32,
}

impl Default for ThreadScroll {
	fn default() -> Self {
		Self {
			following: true,
			pending_follow: 0,
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
	OnSpawn::insert(rsx! {
		<div {message_block(&kind)}>
			<div {author_label(&kind)}>{author}</div>
			// seeded with the body at build time so a new row paints its text on
			// the first frame, bound so streamed growth re-syncs in place
			<div {body_text()}>{(Value::new(text), FieldRef::new("text"))}</div>
		</div>
	})
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
/// [`ThreadView`] watching its thread, and (per the contract) into a [`Document`]
/// on the thread entity itself.
///
/// The document holds every post (display intent or not, so reasoning and tool
/// traffic remain inspectable) as `{ "posts": [{ id, author, text }, ..] }`.
/// Keyed reconciliation downstream means a grown in-progress body updates a row
/// rather than rebuilding it, so streaming flows through the bound [`Value`].
pub(crate) fn project_window_to_document(
	mut commands: Commands,
	windows: Query<
		(Entity, &ThreadWindow, Option<&ThreadItems>),
		Changed<ThreadWindow>,
	>,
	views: Query<(), With<ThreadView>>,
	mut documents: Query<&mut Document>,
) -> Result {
	for (thread_entity, window, items) in windows.iter() {
		let value = project_window(window);
		// the contract's thread-side document, inserted if absent
		set_document(
			&mut commands,
			&mut documents,
			thread_entity,
			value.clone(),
		);
		// every view item of this thread renders against its own co-located document,
		// reached by traversing the thread's `ThreadItems` instead of scanning views.
		items
			.into_iter()
			.flat_map(|items| items.iter())
			.filter(|item| views.contains(*item))
			.for_each(|view_entity| {
				set_document(
					&mut commands,
					&mut documents,
					view_entity,
					value.clone(),
				);
			});
	}
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
	parents: Query<&ChildOf>,
	portals: Query<&PortalOf>,
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
		let path = scroll_path(transcript, &parents, &portals);
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
				scroll.offset.y = i32::MAX;
			}
		}
	}
}

/// How many frames a follow keeps pinning: enough to outlast the row spawn, the
/// layout pass and the clamp that gives the containers their extent.
const FOLLOW_FRAMES: u8 = 4;

/// The transcript and every container it renders inside, innermost first.
///
/// Ancestry is *visual*, so the walk crosses transclusion: a [`PortalOf`] holder
/// is the visual parent of the content it renders in place, which is how a
/// router page reaches the surface scrollport hosting it. The same walk the
/// charcell wheel/key scrolling uses to resolve its container.
fn scroll_path(
	transcript: Entity,
	parents: &Query<&ChildOf>,
	portals: &Query<&PortalOf>,
) -> Vec<Entity> {
	let mut path = vec![transcript];
	let mut current = Some(transcript);
	while let Some(entity) = current {
		current = portals
			.get(entity)
			.ok()
			.and_then(|portal| portal.holders().first().copied())
			.or_else(|| {
				parents.get(entity).ok().map(|child_of| child_of.parent())
			});
		path.extend(current);
	}
	path
}

/// Build the document value for a window: a `posts` list of `{ id, author, text }`.
fn project_window(window: &ThreadWindow) -> Value {
	let posts = window
		.post_views()
		.map(|view| {
			Value::Map(
				[
					("id".into(), Value::new(view.post.id().to_string())),
					("author".into(), Value::new(view.actor.name())),
					("kind".into(), Value::new(post_kind(&view))),
					("text".into(), Value::new(view.post.to_string())),
				]
				.into_iter()
				.collect(),
			)
		})
		.collect::<Vec<_>>();
	Value::Map([("posts".into(), Value::List(posts))].into_iter().collect())
}

/// The display role of a post, used by [`post_row`] for styling: a 5xx error post
/// renders as `error`, otherwise it follows the authoring actor's kind.
fn post_kind(view: &PostView) -> &'static str {
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
	use beet_core::prelude::*;
	// the live-TUI render stack the repaint test paints into
	use beet_router::prelude::*;
	use beet_ui::prelude::CharcellPlugin;
	use beet_ui::prelude::DoubleBuffer;
	use beet_ui::prelude::Portal;
	use beet_ui::prelude::RealtimeParsePlugin;
	use beet_ui::prelude::RenderSurface;
	use bevy::math::UVec2;

	/// The live tail: posts appended *after* the first projection must reach the
	/// view too, both as document entries and as spawned rows. Regression for a
	/// transcript that renders its first batch and then freezes.
	#[beet_core::test]
	fn projects_appended_posts() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<DocumentPlugin>()
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();
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
		app.add_plugins(MinimalPlugins)
			.init_plugin::<DocumentPlugin>()
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();
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

	/// The live-TUI render stack minus the terminal (as `live_page`'s own tests
	/// build it), hosting a page-bound view of a fresh thread: the app, the
	/// thread entity and the host whose buffer is painted.
	fn live_host() -> (App, Entity, Entity) {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			LivePagePlugin,
		))
		.init_plugin::<ThreadPlugin>()
		.init_plugin::<ThreadUiPlugin>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadWindow::default()))
			.id();
		let host = app
			.world_mut()
			.spawn(PageHost::bundle(UVec2::new(40, 12)))
			.id();
		let page = app
			.world_mut()
			.spawn_template(Snippet::from_bundle(rsx! {
				<ThreadView {OfThread(thread)}/>
			}))
			.unwrap()
			.id();
		// bind the page to the host surface (`bind_surface_page` is
		// router-internal; a fresh host has no outgoing page to clean up). The
		// slot is a descendant of the host, a grandchild as `PageHost::bundle` spawns it.
		let mut stack = vec![host];
		let mut slot = None;
		while let Some(entity) = stack.pop() {
			if app.world().get::<PageSlot>(entity).is_some() {
				slot = Some(entity);
				break;
			}
			if let Some(children) = app.world().get::<Children>(entity) {
				stack.extend(children.iter());
			}
		}
		let slot = slot.unwrap();
		app.world_mut().entity_mut(page).insert(RenderSurface(host));
		app.world_mut().entity_mut(slot).insert(Portal::new(page));
		(app, thread, host)
	}

	/// A few frames, enough for the projection, the document chain and the
	/// paint to settle.
	fn settle(app: &mut App) { (0..4).for_each(|_| app.update()); }

	/// The host's painted frame as plain text.
	fn frame(app: &App, host: Entity) -> String {
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.current_buffer()
			.render_plain()
	}

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
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();
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
}
