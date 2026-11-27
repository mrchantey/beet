//! The reactive transcript widget: a window's [`ThreadRecords`] rendered as a
//! scrollable list of posts, one row entity a loaded post, each its own
//! [`Document`].

use crate::prelude::*;
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
use std::collections::BTreeMap;

// ═══════════════════════════════════════════════════════════════════════
// ThreadView: the reactive chat widget
// ═══════════════════════════════════════════════════════════════════════

/// A reactive view of a window, rendering its [`ThreadRecords`] as a scrollable
/// list of posts: one row entity a loaded post, in [`PostId`] order, each
/// carrying its own [`Document`] `{author, kind, status, text}` that the row's
/// fields bind to by ancestry. A change writes the documents of the posts that
/// changed and nothing else ([`ThreadRecords::changed_since`]), so streaming a
/// token costs one document however long the thread is, and a row keeps its
/// entity while its post streams.
///
/// Scrolled near its oldest row, the view reads the window's older posts
/// ([`ThreadStore::read_older`]), keeping the reader's place.
///
/// Host-agnostic content, not a host: spawn it under any render surface (a
/// charcell terminal host, a web page) and bind it to its window with an
/// [`OfThread`] relationship. A marker, so the bound window lives in the
/// relationship, not a stored field. From markup the view is the tag and its
/// binding a spread on the same entity, so the same view serves a local terminal
/// and a per-connection server surface alike:
///
/// ```rsx
/// <Thread bx:ref="thread" {Sequence}>..</Thread>
/// <ThreadView {OfThread($thread)}/>
/// ```
///
/// Log posts render as dividers, a join naming its actor. The view reads
/// what is shown ([`ThreadRecords::shown`]): an edited post shows its new
/// body and that it was edited, a deleted one that it was, with no body, and
/// neither log is a row of its own. A view whose page also holds a composer
/// of its window ([`CreatePostForm`]) is bound to it: the composer's actor's
/// own posts carry `edit` and `delete`, which the composer acts on.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ThreadRows)]
#[component(on_add = thread_view_on_add)]
pub struct ThreadView;

impl ThreadView {
	/// A view bound to `thread`. Its scroll container is attached in `on_add`,
	/// so the bundle works both as a direct spawn and as a markup spread.
	pub fn new(thread: Entity) -> impl Bundle { (Self, OfThread(thread)) }
}

/// A [`ThreadView`]'s rows: the row entity of each loaded post, and the
/// window and revision of its records the view last drew.
#[derive(Debug, Default, Component)]
pub struct ThreadRows {
	/// The window drawn, so a view bound to another draws it afresh.
	window: Option<Entity>,
	/// The revision of the window's records last drawn.
	read: Revision,
	/// The row entity of each loaded post.
	rows: BTreeMap<PostId, Entity>,
	/// The `edit` and `delete` controls of each row that carries them.
	controls: BTreeMap<PostId, Entity>,
}

impl ThreadRows {
	/// The row entity of `post`, if the view drew it.
	pub fn row(&self, post: PostId) -> Option<Entity> {
		self.rows.get(&post).copied()
	}
}

/// One row of a [`ThreadView`], the post it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct ThreadRow(pub PostId);

/// Merge the scroll container onto the view when a [`ThreadView`] is added,
/// so the view *is* the scrolling transcript its rows are children of.
fn thread_view_on_add(mut world: DeferredWorld, cx: HookContext) {
	// All styling goes through the cascade (`inline_class!`): `resolve_styles`
	// rebuilds `LayoutStyle`/`BoxStyle` from the cascade for every node of a
	// styled tree, so a set component would be clobbered.
	world.commands().entity(cx.entity).insert(rsx! {
		<div {(
			ThreadScroll::default(),
			ScrollPosition::default(),
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
/// `follow_thread_scroll` uses to keep the latest post in view and a scrolled
/// up reader's place.
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ThreadScroll {
	/// Whether the transcript tails the latest post. On by default, off while
	/// the reader has scrolled up, back on when they return to the bottom.
	following: bool,
	/// Frames of pinning left to apply, counted down as the follow re-asserts
	/// itself over the frames the new rows take to lay out.
	pending_follow: u8,
	/// Frames left to hold a scrolled up reader's place while older rows
	/// land above it and lay out.
	keep_place: u8,
	/// The scrollable extent last seen, so growing content is not mistaken for
	/// the reader scrolling.
	last_max: i32,
}

impl Default for ThreadScroll {
	fn default() -> Self {
		Self {
			following: true,
			pending_follow: 0,
			keep_place: 0,
			last_max: 0,
		}
	}
}

/// The document of the row showing `view`, a post as shown:
/// `{author, label, kind, status, text}`, `label` the author line, which
/// says when the post was `edited`, and `kind` the row's display role
/// ([`row_kind`]).
fn row_document(view: &AuthoredPost, edited: bool) -> Document {
	let status = view.post.status().as_str();
	let label = match edited && !view.post.is_deleted() {
		true => format!("{} (edited)", view.actor.name()),
		false => view.actor.name().to_string(),
	};
	Document::new(Value::Map(
		[
			("author".into(), Value::new(view.actor.name())),
			("label".into(), Value::new(label)),
			("kind".into(), Value::new(row_kind(view))),
			("status".into(), Value::new(status)),
			("text".into(), Value::new(row_text(view))),
		]
		.into_iter()
		.collect(),
	))
}

/// Build one post row from its document: a role-accented block holding the
/// author label and the post body, each bound through a [`FieldRef`] to the
/// row's own document, so streamed text re-syncs in place. Styling is keyed
/// on the row's `kind` (`system`/`user`/`agent`/`error`), which a post never
/// changes, so each speaker reads distinctly.
fn post_row(document: &Document) -> OnSpawn {
	let field = |name: &str| {
		document
			.get(name)
			.and_then(|value| value.as_str().ok())
			.unwrap_or_default()
			.to_string()
	};
	let (label, kind, text) = (field("label"), field("kind"), field("text"));
	// each seeded with its value at build time so a new row paints on the
	// first frame, bound so a change re-syncs in place
	if kind == LOG_ROW {
		return OnSpawn::insert(rsx! {
			<div {divider_style()}>{(Value::new(text), FieldRef::new("text"))}</div>
		});
	}
	OnSpawn::insert(rsx! {
		<div {message_block(&kind)}>
			<div {author_label(&kind)}>{(Value::new(label), FieldRef::new("label"))}</div>
			<div {body_text()}>{(Value::new(text), FieldRef::new("text"))}</div>
		</div>
	})
}

/// A log post: a dim, centered rule, ie an actor joining.
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
// Rows: ThreadRecords -> one Document a post
// ═══════════════════════════════════════════════════════════════════════

/// Draw what changed in each view's window since the view last looked: a
/// changed post's row has its [`Document`] written from the post as shown,
/// a new post gets a row in [`PostId`] order, and nothing else is touched;
/// an `edited` or `deleted` log gets none, the post it names showing the
/// change. A row the view's composer may change carries its controls while
/// it may. A view bound to another window draws it afresh.
pub(crate) fn sync_thread_rows(
	mut commands: Commands,
	windows: Query<(&Thread, &ThreadRecords)>,
	composers: ViewComposer,
	mut views: Query<
		(
			Entity,
			&OfThread,
			&mut ThreadRows,
			Option<&mut ThreadScroll>,
		),
		With<ThreadView>,
	>,
	mut documents: Query<&mut Document, With<ThreadRow>>,
) {
	for (view, of_thread, mut rows, mut thread_scroll) in views.iter_mut() {
		let window = of_thread.thread();
		let Ok((thread, records)) = windows.get(window) else {
			continue;
		};
		if rows.window != Some(window) {
			for row in core::mem::take(&mut rows.rows).into_values() {
				commands.entity(row).despawn();
			}
			rows.controls.clear();
			rows.window = Some(window);
			rows.read = Revision::default();
		}
		if rows.read == records.revision() {
			continue;
		}
		let since = core::mem::replace(&mut rows.read, records.revision());
		// the actor whose posts carry controls, its composer's
		let editor = composers
			.of(view, window)
			.and_then(|form| records.author(form.actor.as_ref()).ok());
		let mut spawned = Vec::new();
		let mut controlled = Vec::new();
		for (id, _) in records
			.changed_since(since)
			.filter(|(_, change)| change.post_after(since))
		{
			let Some(post) = records.authored(id) else {
				continue;
			};
			if matches!(post.kind(), PostKind::Edited | PostKind::Deleted) {
				continue;
			}
			let shown = records.shown(id..=id);
			let shown = AuthoredPost {
				post: &shown[0],
				actor: post.actor,
			};
			let document = row_document(&shown, records.was_edited(id));
			let controls = editor
				.as_ref()
				.filter(|editor| post.check_change(editor, thread.id()).is_ok())
				.map(|_| post.kind() == &PostKind::Text);
			controlled.push((id, controls));
			match rows.rows.get(&id) {
				Some(row) => {
					if let Ok(mut existing) = documents.get_mut(*row) {
						existing.set_if_neq(document);
					}
				}
				None => spawned.push((id, document)),
			}
		}
		// a row landing above one already drawn shifts what a scrolled up
		// reader sees
		let newest_drawn = rows.rows.keys().next_back().copied();
		if spawned
			.iter()
			.any(|(id, _)| newest_drawn.is_some_and(|newest| *id < newest))
			&& let Some(thread_scroll) = thread_scroll.as_mut()
		{
			thread_scroll.keep_place = FOLLOW_FRAMES;
		}
		// newest first, so each row's successor is already in place
		spawned.sort_by_key(|(id, _)| core::cmp::Reverse(*id));
		for (id, document) in spawned {
			let next = rows
				.rows
				.range((
					core::ops::Bound::Excluded(id),
					core::ops::Bound::Unbounded,
				))
				.next()
				.map(|(_, row)| *row);
			let row = commands
				.spawn((ThreadRow(id), post_row(&document), document))
				.id();
			rows.rows.insert(id, row);
			commands.queue(move |world: &mut World| {
				place_row(world, view, row, next);
			});
		}
		// controls follow whether the composer may change the post, added
		// once the row's own content is in place
		for (id, controls) in controlled {
			match (controls, rows.controls.get(&id).copied()) {
				(Some(edit), None) => {
					let Some(row) = rows.row(id) else {
						continue;
					};
					let entity = commands.spawn_empty().id();
					rows.controls.insert(id, entity);
					commands.queue(move |world: &mut World| {
						if world.get_entity(row).is_ok()
							&& let Ok(mut entity) = world.get_entity_mut(entity)
						{
							entity.insert((ChildOf(row), row_controls(edit)));
						}
					});
				}
				(None, Some(entity)) => {
					rows.controls.remove(&id);
					commands.entity(entity).try_despawn();
				}
				_ => {}
			}
		}
	}
}

/// The `edit` and `delete` controls of a row, `edit` for a text post alone,
/// since the composer takes text.
fn row_controls(edit: bool) -> impl Bundle {
	let delete = rsx! {
		<button type="button" {(control_style(), OnSpawn::observe(delete_on_click))}>"delete"</button>
	};
	match edit {
		true => OnSpawn::insert(rsx! {
			<div {controls_style()}>
				<button type="button" {(control_style(), OnSpawn::observe(edit_on_click))}>"edit"</button>
				{delete}
			</div>
		}),
		false => OnSpawn::insert(rsx! {
			<div {controls_style()}>{delete}</div>
		}),
	}
}

/// A row's controls: a gapped row of small buttons.
fn controls_style() -> OnSpawn {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Horizontal
		),
		(style::common_props::ColumnGapProp, style::Length::Rem(1.)),
	]
}

/// A row's control: dim until focused, beside the post it changes.
fn control_style() -> OnSpawn {
	inline_class![Declaration::token(
		style::common_props::ForegroundColor,
		colors::Outline
	),]
}

/// Place `row` among `view`'s children before its successor `next`, else
/// last.
fn place_row(
	world: &mut World,
	view: Entity,
	row: Entity,
	next: Option<Entity>,
) {
	let index = next.and_then(|next| {
		world.get::<Children>(view).and_then(|children| {
			children.iter().position(|child| child == next)
		})
	});
	let Ok(mut view) = world.get_entity_mut(view) else {
		return;
	};
	match index {
		Some(index) => view.insert_children(index, &[row]),
		None => view.add_child(row),
	};
}

/// Read a window's older posts once a view of it is scrolled near its oldest
/// row, the reader not tailing the latest: one read at a time a window
/// ([`ReadingOlder`]), and none while the rows of the last are still landing,
/// before the view has drawn them and held the reader's place.
pub(crate) fn read_older_on_scroll(
	mut commands: AsyncCommands,
	views: Query<
		(Entity, &OfThread, &ThreadScroll, &ThreadRows),
		With<ThreadView>,
	>,
	windows: Query<&ThreadRecords, (With<ThreadStore>, Without<ReadingOlder>)>,
	tree: RenderTreeQuery,
	scrolls: Query<&ScrollPosition>,
) {
	for (view, of_thread, thread_scroll, rows) in views.iter() {
		let window = of_thread.thread();
		if thread_scroll.keep_place > 0
			|| !windows.get(window).is_ok_and(|records| {
				records.has_older() && records.revision() == rows.read
			}) {
			continue;
		}
		// the innermost container with somewhere to scroll, as the follow
		// finds it
		let near_oldest = !thread_scroll.following
			&& tree
				.iter_ancestors_inclusive(view)
				.filter_map(|entity| scrolls.get(entity).ok())
				.find(|scroll| scroll.max.y > 0)
				.is_some_and(|scroll| scroll.offset.y <= READ_OLDER_WITHIN);
		if near_oldest {
			commands.entity(window).insert(ReadingOlder);
			commands.run(async move |world: AsyncWorld| -> Result {
				let window = world.entity(window);
				let read = ThreadStore::read_older_into(&window).await;
				window.take::<ReadingOlder>().await?;
				read.map(|_| ())
			});
		}
	}
}

/// How near its top, in cells, a view's scroll reads older posts.
const READ_OLDER_WITHIN: i32 = 2;

/// Marks a window reading its older posts, so a view asks once at a time.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct ReadingOlder;

/// Follow-on-append: when a [`ThreadView`]'s rows change (a post was added
/// or grew) *or* its rows spawn in, arm a follow, then for the next
/// few frames pin every scroll container showing the transcript to the bottom
/// by parking its offset past the end. `clamp_scroll_positions` re-clamps that
/// to the true max against the laid-out content.
///
/// The follow spans frames rather than landing in one because the geometry
/// trails the content: rows spawn, then lay
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
/// who scrolled away, and stop following on the first burst. Rows landing
/// above such a reader, older posts read in, grow the extent above them, and
/// the follow moves the reader's offset by that growth, so the rows they were
/// reading stay put.
pub(crate) fn follow_thread_scroll(
	views: Query<
		(Entity, Ref<ThreadRows>, Option<Ref<Children>>),
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
	for (view, drawn, rows) in views.iter() {
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
			drawn.is_changed() || rows.is_some_and(|rows| rows.is_changed());
		let Ok(mut thread_scroll) = transcripts.get_mut(transcript) else {
			continue;
		};
		// rows landed above a reader who scrolled up: move them by the growth
		if !thread_scroll.following && thread_scroll.keep_place > 0 {
			thread_scroll.keep_place -= 1;
			let grown = max - thread_scroll.last_max;
			if grown != 0
				&& let Some(mut scroll) = path
					.iter()
					.find(|entity| {
						scrolls
							.get(**entity)
							.is_ok_and(|scroll| scroll.max.y > 0)
					})
					.and_then(|entity| scrolls.get_mut(*entity).ok())
			{
				scroll.offset.y += grown;
			}
			thread_scroll.last_max = max;
			continue;
		}
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

/// The text a row shows: a log post as a divider line, a join naming its
/// actor, a compaction how far it covers, never its text, and any other kind
/// by its name, and any other post as its body.
fn row_text(view: &AuthoredPost) -> String {
	if view.post.is_deleted() {
		return "(deleted)".to_string();
	}
	if SessionOpened::opens(view.post) {
		return format!("── {} joined ──", view.actor.name());
	}
	if view.post.kind() == &PostKind::Compaction {
		let through = view
			.post
			.subject()
			.map(|subject| {
				format!(" up to {}", subject.created().format_iso8601_mins())
			})
			.unwrap_or_default();
		return format!(
			"── {} summarised the conversation{through} ──",
			view.actor.name()
		);
	}
	if view.post.is_log() {
		return format!("── {} ──", view.post.kind());
	}
	view.post.to_string()
}

/// The row kind of a log post, which [`post_row`] renders as a divider.
const LOG_ROW: &str = "log";

/// The display role of a post, used by [`post_row`] for styling: a log post
/// renders as [`LOG_ROW`], an error post as `error`, otherwise it follows the
/// authoring actor's kind.
fn row_kind(view: &AuthoredPost) -> &'static str {
	match view.post.kind() {
		kind if kind.is_log() => return LOG_ROW,
		PostKind::Error => return "error",
		_ => {}
	}
	match view.actor.kind() {
		ActorKind::System => "system",
		ActorKind::Developer => "developer",
		ActorKind::User => "user",
		ActorKind::Agent => "agent",
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::ui::test_ext;
	use beet_core::prelude::*;
	use beet_scene::prelude::*;
	use bevy::math::UVec2;

	/// A log post's row is a divider in its own words, beside the posts of
	/// the conversation, a compaction saying how far it covers and never its
	/// text.
	#[beet_core::test]
	fn log_posts_are_dividers() {
		let mut records = ThreadRecords::new();
		let thread = ThreadId::now();
		let user = records.insert_actor(Actor::user());
		records.upsert_post(AgentPost::new_text(
			user.clone(),
			thread,
			"hello",
			PostStatus::Completed,
		));
		records.join(thread, [user], &default()).unwrap();
		let hello = records.posts()[0].id();
		records.insert_actor(Actor::agent());
		records.upsert_post(
			Post::new_log(ActorId::AGENT, thread, &Compaction {
				text: "SUMMARY-TEXT".into(),
				..default()
			})
			.unwrap()
			.with_subject(hello),
		);
		let rows = |field: &str| {
			records
				.authored_posts()
				.map(|view| {
					super::row_document(&view, false)
						.get(field)
						.unwrap()
						.as_str()
						.unwrap()
						.to_string()
				})
				.collect::<Vec<_>>()
		};
		rows("kind").xpect_eq(vec![
			"user".to_string(),
			"log".to_string(),
			"log".to_string(),
		]);
		rows("text").xpect_eq(vec![
			"hello".to_string(),
			"── User joined ──".to_string(),
			format!(
				"── Agent summarised the conversation up to {} ──",
				hello.created().format_iso8601_mins()
			),
		]);
	}

	/// The live tail: posts appended *after* the first rows are drawn reach
	/// the view too, as spawned rows. Regression for a transcript that
	/// renders its first batch and then freezes.
	#[beet_core::test]
	fn draws_appended_posts() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).install_types::<(
			DocumentPlugin,
			ThreadPlugin,
			ThreadUiPlugin,
		)>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadRecords::default()))
			.id();
		let view = app.world_mut().spawn(ThreadView::new(thread)).id();
		app.update();

		// rows are drawn in `Update`, their commands applied after it
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
	fn draws_appended_posts_through_template() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).install_types::<(
			DocumentPlugin,
			ThreadPlugin,
			ThreadUiPlugin,
		)>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadRecords::default()))
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
	/// frame is repainted, not just drawn as a row.
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
	/// when the very first drawing already overflows (a feed's opening page,
	/// rather than a chat's one-post-at-a-time growth).
	#[beet_core::test]
	fn follows_the_tail_when_overflowing() {
		let (mut app, thread, host) = live_host();
		// a burst that overflows the 12 row viewport in one drawing
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

	/// A streamed token over 10,000 loaded posts writes the one document of
	/// the post it grows, and the row keeps its entity as its post streams
	/// and completes.
	#[beet_core::test]
	fn a_token_writes_one_document() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).install_types::<(
			DocumentPlugin,
			ThreadPlugin,
			ThreadUiPlugin,
		)>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadRecords::default()))
			.id();
		let view = app.world_mut().spawn(ThreadView::new(thread)).id();
		for index in 0..10_000 {
			push_post(&mut app, thread, &format!("post {index}"));
		}
		let start = Instant::now();
		app.update();
		app.update();
		let drawn = start.elapsed();
		row_count(&app, view).xpect_eq(10_000);

		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let mut streaming = AgentPost::new_text(
			ActorId::AGENT,
			thread_id,
			"hel",
			PostStatus::InProgress,
		);
		let upsert = |app: &mut App, post: &Post| {
			app.world_mut()
				.get_mut::<ThreadRecords>(thread)
				.unwrap()
				.upsert_post(post.clone());
		};
		upsert(&mut app, &streaming);
		app.update();
		app.update();
		let row = |app: &App, id| {
			app.world()
				.get::<ThreadRows>(view)
				.unwrap()
				.row(id)
				.unwrap()
		};
		let streamed_row = row(&app, streaming.id());
		// the tick each row's document last changed at, before the token
		let ticks = |app: &mut App| {
			app.world_mut()
				.query_filtered::<(Entity, Ref<Document>), With<ThreadRow>>()
				.iter(app.world())
				.map(|(entity, document)| (entity, document.last_changed()))
				.collect::<HashMap<_, _>>()
		};
		let before = ticks(&mut app);
		streaming.set_text("hello");
		upsert(&mut app, &streaming);
		let start = Instant::now();
		app.update();
		let token = start.elapsed();
		let after = ticks(&mut app);
		after
			.iter()
			.filter(|(entity, tick)| before.get(*entity) != Some(*tick))
			.map(|(entity, _)| *entity)
			.collect::<Vec<_>>()
			.xpect_eq(vec![streamed_row]);
		info!("10,000 rows drawn in {drawn:?}, a token in {token:?}");

		streaming.set_status(PostStatus::Completed);
		upsert(&mut app, &streaming);
		app.update();
		row(&app, streaming.id()).xpect_eq(streamed_row);
		app.world()
			.get::<Document>(streamed_row)
			.unwrap()
			.get("status")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("completed");
	}

	/// A page-bound view of a fresh thread in the live-TUI render stack: the
	/// app, the thread entity and the host whose buffer is painted.
	fn live_host() -> (App, Entity, Entity) {
		let mut app = test_ext::render_app();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ThreadRecords::default()))
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

	/// Append one completed text post to the thread's records.
	fn push_post(app: &mut App, thread: Entity, text: &str) {
		let thread_id = app.world().get::<Thread>(thread).unwrap().id();
		let mut records =
			app.world_mut().get_mut::<ThreadRecords>(thread).unwrap();
		let actor = records.insert_actor(Actor::agent());
		records.upsert_post(AgentPost::new_text(
			actor,
			thread_id,
			text,
			PostStatus::Completed,
		));
	}

	/// The rows the view has spawned for its posts.
	fn row_count(app: &App, view: Entity) -> usize {
		app.world()
			.entity(view)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.filter(|child| {
						app.world().get::<ThreadRow>(*child).is_some()
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
			.install_types::<(ThreadPlugin, ThreadUiPlugin)>();
		let source = r#"
<Fragment>
	<Thread bx:ref="thread" {Sequence}/>
	<ThreadView {OfThread($thread)}/>
</Fragment>
"#;
		BsxParser::spawn(app.world_mut(), source).unwrap();
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
