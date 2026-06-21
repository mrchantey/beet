//! Agnostic reactive chat UI for a [`ThreadWindow`], built on `beet_ui`.
//!
//! The pipeline is two hops, each renderer-agnostic:
//! 1. [`project_window_to_document`] projects a thread's [`ThreadWindow`] into a
//!    structured [`Document`] (`{ "posts": [{ id, author, text }, ..] }`),
//! 2. [`ThreadView`] renders that document reactively: a scroll container whose
//!    rows are a *keyed* [`ReactiveChildren`] over the `posts` field, each row's
//!    body bound through a [`FieldRef`] so streamed text flows in without
//!    rebuilding the row.
//!
//! Keying on the post id means an appended post reuses every settled row's
//! entity and binding, and a growing in-progress body re-syncs that row's bound
//! [`Value`] rather than respawning it. `beet_ui` never depends on `beet_thread`;
//! this layer is additive, behind the `ui` feature.

use crate::prelude::*;
// `Table::id()` on a `Post` (via `PostView`'s deref); the `beet_ui` glob below
// otherwise shadows the prelude re-export of this trait.
use crate::table::Table;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::style::LayoutStyle;
use beet_ui::prelude::*;

/// Registers the [`ThreadWindow`] -> [`Document`] projection and the reactive
/// UI types. Pairs with `beet_ui`'s [`CharcellTuiPlugin`] (or any renderer that
/// drives the document chain) and the [`ThreadPlugin`].
#[derive(Default)]
pub struct ThreadUiPlugin;

impl Plugin for ThreadUiPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<ThreadView>()
			.register_type::<ThreadScroll>()
			.register_type::<ThreadComposer>()
			// project first, then follow the resulting document change to the bottom
			.add_systems(
				Update,
				(
					reply_to_user_posts,
					(project_window_to_document, follow_thread_scroll).chain(),
				),
			)
			// a composer submit appends a user post to its thread's window
			.add_observer(submit_composer);
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Terminal hosts: compose a charcell host around a thread
// ═══════════════════════════════════════════════════════════════════════

/// A scrolling charcell transcript of `thread`, for non-interactive runs (a
/// finite agent task or an auto agent-to-agent loop). Inline mode keeps the
/// rendered transcript in the terminal scrollback after the process exits.
///
/// Spawn it in a startup system; `CharcellTuiPlugin` paints it to the real
/// terminal. Pair with [`ThreadView`]'s plugin via [`ThreadUiPlugin`].
pub fn thread_tui(thread: Entity) -> impl Bundle {
	(
		StdioTerminal::inline(),
		DoubleBuffer::default(),
		LayoutStyle::flex_col(),
		children![ThreadView::new(thread)],
	)
}

/// A full-screen charcell chat around `thread`: the scrolling transcript above a
/// [`ThreadComposer`] input. Submitting the composer appends a user post, which
/// [`reply_to_user_posts`] answers, so the loop needs no blocking stdin.
pub fn thread_chat_tui(thread: Entity) -> impl Bundle {
	(
		StdioTerminal::default(),
		DoubleBuffer::default(),
		LayoutStyle::flex_col(),
		children![ThreadView::new(thread), ThreadComposer::new(thread)],
	)
}

// ═══════════════════════════════════════════════════════════════════════
// Event-driven turns: an agent answers a user post
// ═══════════════════════════════════════════════════════════════════════

/// When a thread's window gains a user-authored latest post, run each of its
/// agents' turns once. This drives interactive chat without a blocking input
/// action: the [`ThreadComposer`] (or a loaded history) appends the user post,
/// and the agent answers reactively.
///
/// It is self-gating against re-entry: once an agent starts replying the latest
/// post is the agent's, so a streaming turn never re-triggers itself; auto and
/// finite threads (no `User` actor) never fire it.
pub fn reply_to_user_posts(
	async_commands: AsyncCommands,
	windows: Query<(Entity, &ThreadWindow), Changed<ThreadWindow>>,
	children: Query<&Children>,
	agents: Query<(Entity, &ActorRef), With<Action<(), Outcome>>>,
) {
	for (thread, window) in windows.iter() {
		// only answer when the user just spoke
		let user_spoke = window
			.last_post()
			.and_then(|post| window.actor(post.author()).ok())
			.map(|actor| actor.kind() == ActorKind::User)
			.unwrap_or(false);
		if !user_spoke {
			continue;
		}
		// run every agent actor under this thread, in document order
		for (agent, _) in children
			.iter_descendants(thread)
			.filter_map(|entity| agents.get(entity).ok())
			.filter(|(_, actor_ref)| {
				window
					.actor(actor_ref.0)
					.map(|actor| actor.kind() == ActorKind::Agent)
					.unwrap_or(false)
			}) {
			async_commands.run(async move |world: AsyncWorld| -> Result {
				world.entity(agent).call::<(), Outcome>(()).await?;
				Ok(())
			});
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// ThreadView: the reactive chat widget
// ═══════════════════════════════════════════════════════════════════════

/// A reactive view of a thread, rendering its [`ThreadWindow`] as a scrollable
/// list of posts. Carries its own [`Document`] (seeded empty, filled by
/// [`project_window_to_document`]), so the inner [`ReactiveChildren`] and the
/// per-row [`FieldRef`]s resolve against it via `DocumentPath::Ancestor`.
///
/// Spawn it anywhere in a render tree (eg under a charcell host) and point it at
/// the thread entity carrying the window:
///
/// ```no_run
/// # use beet_thread::prelude::*;
/// # use beet_core::prelude::*;
/// # let mut world = World::new();
/// # let thread = world.spawn_empty().id();
/// world.spawn(ThreadView::new(thread));
/// ```
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
#[require(Document)]
pub struct ThreadView {
	/// The thread entity whose [`ThreadWindow`] this view renders.
	pub thread: Entity,
}

impl ThreadView {
	/// A view of `thread`, with the reactive post list as its content.
	pub fn new(thread: Entity) -> impl Bundle {
		(ThreadView { thread }, Self::content())
	}

	/// The reactive content tree: a scroll container whose children are one row
	/// per `posts` item, keyed by post id so appends reuse settled rows.
	fn content() -> impl Bundle {
		rsx! {
			<div {(
				ThreadScroll,
				ScrollPosition::default(),
				FieldRef::new("posts"),
				ReactiveChildren::keyed(post_key, post_row),
			)}/>
		}
	}
}

/// Marks a [`ThreadView`]'s scroll container, so [`follow_thread_scroll`] can
/// pin it to the latest post on append.
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct ThreadScroll;

/// Stable reconciliation key for a post item: its `id` field (a uuid string), so
/// reconciliation reuses a row across appends and in-progress body growth.
fn post_key(item: &Value) -> String {
	item.get("id")
		.and_then(|id| id.as_str().ok())
		.unwrap_or_default()
		.to_string()
}

/// Build one post row: the author name, then the post body bound through a
/// [`FieldRef`] so streamed text re-syncs in place. The row's terminating scope
/// is `posts[index]`, so `text` resolves to `posts[index].text`.
fn post_row(_index: usize, item: &Value) -> OnSpawn {
	let author = item
		.get("author")
		.and_then(|author| author.as_str().ok())
		.unwrap_or_default()
		.to_string();
	OnSpawn::insert(rsx! {
		<div>
			<span>{author}": "</span>
			<span>{(Value::default(), FieldRef::new("text"))}</span>
		</div>
	})
}

// ═══════════════════════════════════════════════════════════════════════
// ThreadComposer: agnostic input
// ═══════════════════════════════════════════════════════════════════════

/// An agnostic chat composer: a form whose submit appends a user post to its
/// thread's [`ThreadWindow`]. Terminal and web both drive it through `beet_ui`'s
/// form + focus-input machinery, the cross-platform input for a thread (no
/// blocking stdin read).
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct ThreadComposer {
	/// The thread entity whose window a submitted message is appended to.
	pub thread: Entity,
}

impl ThreadComposer {
	/// A composer for `thread`, with a text field + send button as its content.
	pub fn new(thread: Entity) -> impl Bundle {
		(ThreadComposer { thread }, Self::content())
	}

	/// A `<form>` whose `message` field + submit button fire `beet_ui`'s
	/// [`Submit`], handled by [`submit_composer`].
	fn content() -> impl Bundle {
		rsx! {
			<form>
				<input name="message" type="text"/>
				<button>"Send"</button>
			</form>
		}
	}
}

/// On a composer's [`Submit`], append the typed `message` as a user post in the
/// thread's window, authored by the thread's first `User`-kind actor.
fn submit_composer(
	ev: On<Submit>,
	parents: Query<&ChildOf>,
	composers: Query<&ThreadComposer>,
	threads: Query<&Thread>,
	mut windows: Query<&mut ThreadWindow>,
) -> Result {
	// the submitted form belongs to the composer it descends from
	let Some(composer) = parents
		.iter_ancestors_inclusive(ev.form)
		.find_map(|ancestor| composers.get(ancestor).ok())
	else {
		return Ok(());
	};
	let text = ev
		.values
		.get("message")
		.and_then(|message| message.as_str().ok())
		.unwrap_or_default()
		.to_string();
	if text.is_empty() {
		return Ok(());
	}

	let thread_id = threads.get(composer.thread)?.id();
	let mut window = windows.get_mut(composer.thread)?;
	let Some(user) = window
		.actors()
		.values()
		.find(|actor| actor.kind() == ActorKind::User)
		.map(|actor| actor.id())
	else {
		return Ok(());
	};
	window.upsert_post(AgentPost::new_text(
		user,
		thread_id,
		text,
		PostStatus::Completed,
	));
	Ok(())
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
pub fn project_window_to_document(
	mut commands: Commands,
	windows: Query<(Entity, &ThreadWindow), Changed<ThreadWindow>>,
	views: Query<(Entity, &ThreadView)>,
	mut documents: Query<&mut Document>,
) -> Result {
	for (thread_entity, window) in windows.iter() {
		let value = project_window(window);
		// the contract's thread-side document, inserted if absent
		set_document(
			&mut commands,
			&mut documents,
			thread_entity,
			value.clone(),
		);
		// every view of this thread renders against its own co-located document
		views
			.iter()
			.filter(|(_, view)| view.thread == thread_entity)
			.for_each(|(view_entity, _)| {
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

/// Follow-on-append: when a [`ThreadView`]'s document changes (a post was added
/// or grew), pin its [`ThreadScroll`] container to the bottom by parking the
/// offset past the end. `clamp_scroll_positions` re-clamps it to the true max
/// next frame, against the freshly laid-out content.
pub fn follow_thread_scroll(
	views: Query<Entity, (With<ThreadView>, Changed<Document>)>,
	children: Query<&Children>,
	mut scrolls: Query<&mut ScrollPosition, With<ThreadScroll>>,
) {
	for view in views.iter() {
		for descendant in children.iter_descendants(view) {
			if let Ok(mut scroll) = scrolls.get_mut(descendant) {
				scroll.offset.y = i32::MAX;
			}
		}
	}
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
					("text".into(), Value::new(view.post.to_string())),
				]
				.into_iter()
				.collect(),
			)
		})
		.collect::<Vec<_>>();
	Value::Map([("posts".into(), Value::List(posts))].into_iter().collect())
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
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
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

	/// End to end: a user post drives the agent's reply through
	/// [`reply_to_user_posts`], and both project into the view's document and
	/// render as charcell text, streamed body included.
	#[beet_core::test]
	async fn renders_window_posts() {
		let (mut app, host) = charcell_app();

		// author an ephemeral thread; the user seed makes the agent next to speak
		let thread = app
			.world_mut()
			.spawn((Thread::default(), children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		// reduce (First) then let the driver (Update) queue the agent turn
		app.update();

		// attach the reactive view of that thread under the charcell host
		app.world_mut()
			.entity_mut(host)
			.insert(ThreadView::new(thread));

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

	/// The chat layout ([`thread_chat_tui`]'s structure): a [`ThreadView`] and a
	/// [`ThreadComposer`] as siblings under the host still render the transcript,
	/// so the view works as a child alongside the input, not only on the host.
	#[beet_core::test]
	async fn chat_layout_renders_transcript() {
		let (mut app, host) = charcell_app();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();
		app.world_mut().entity_mut(host).insert((
			LayoutStyle::flex_col(),
			children![ThreadView::new(thread), ThreadComposer::new(thread)],
		));
		for _ in 0..40 {
			app.update();
		}
		let frame = frame_plain(&app, host);
		frame.as_str().xpect_contains("User: hello");
		frame.xpect_contains("Agent: you said: hello");
	}

	/// A composer submit appends a user post to the thread window, authored by
	/// the thread's user actor: the agnostic replacement for blocking stdin. The
	/// focus/typing/button path is `beet_ui`'s (tested there); here the `Submit`
	/// is fired directly to verify the thread-side handler.
	#[beet_core::test]
	fn composer_submits_user_post() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();

		// a thread whose roster includes a user actor (reduced into the window)
		let thread = app
			.world_mut()
			.spawn((Thread::default(), children![
				(Actor::user(), children![Post::spawn("seed")]),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();

		// a composer for the thread, plus a stand-in form descendant to submit
		let composer = app.world_mut().spawn(ThreadComposer { thread }).id();
		let form = app.world_mut().spawn(ChildOf(composer)).id();
		let values = Value::Map(
			[("message".into(), Value::new("hi there"))]
				.into_iter()
				.collect(),
		);
		app.world_mut().trigger(Submit { form, values });
		app.world_mut().flush();

		// the window gained a user-authored post with the typed text
		app.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.post_views()
			.filter(|view| view.actor.kind() == ActorKind::User)
			.any(|view| view.post.to_string() == "hi there")
			.xpect_true();
	}
}
