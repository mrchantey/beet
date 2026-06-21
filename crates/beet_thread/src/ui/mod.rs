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
			.register_type::<UserInput>()
			// project each window into its views' documents, then pin to the bottom
			.add_systems(
				Update,
				(project_window_to_document, follow_thread_scroll).chain(),
			)
			// scope + focus a charcell chat composer (the router's page-host does
			// this for web; a directly-spawned terminal host needs it here)
			.add_observer(focus_chat_composer)
			// empty the composer's field once its message is submitted
			.add_observer(clear_composer_on_submit);
	}
}

/// Clear a [`ThreadComposer`]'s text field after it submits, so the next turn
/// starts from an empty input (the submitted value is already gathered into the
/// [`Submit`], so clearing here never drops it).
fn clear_composer_on_submit(
	ev: On<Submit>,
	parents: Query<&ChildOf>,
	composers: Query<(), With<ThreadComposer>>,
	children: Query<&Children>,
	elements: Query<&Element>,
	mut values: Query<&mut Value>,
) {
	// only forms belonging to a ThreadComposer
	if !parents
		.iter_ancestors_inclusive(ev.form)
		.any(|ancestor| composers.contains(ancestor))
	{
		return;
	}
	std::iter::once(ev.form)
		.chain(children.iter_descendants(ev.form))
		.filter(|entity| {
			elements
				.get(*entity)
				.map(|element| matches!(element.tag(), "input" | "textarea"))
				.unwrap_or(false)
		})
		.for_each(|input| {
			if let Ok(mut value) = values.get_mut(input) {
				*value = Value::str("");
			}
		});
}

/// When a [`ThreadComposer`]'s text input is added under a charcell host, scope
/// the composer to that host surface and focus the input, so typing and Enter
/// reach it. `thread_chat_tui` spawns the host directly, skipping the router's
/// page-host wiring (`RenderSurface` + focus) that would otherwise do this; the
/// web path keeps its own wiring (no [`DoubleBuffer`] host, so this is a no-op).
fn focus_chat_composer(
	ev: On<Add, Element>,
	elements: Query<&Element>,
	parents: Query<&ChildOf>,
	composers: Query<(), With<ThreadComposer>>,
	hosts: Query<(), With<DoubleBuffer>>,
	mut commands: Commands,
) {
	// only the composer's text input
	let Ok(element) = elements.get(ev.entity) else {
		return;
	};
	if !matches!(element.tag(), "input" | "textarea") {
		return;
	}
	let Some(composer) = parents
		.iter_ancestors_inclusive(ev.entity)
		.find(|ancestor| composers.contains(*ancestor))
	else {
		return;
	};
	// charcell only: the terminal host (the window keyboard events carry)
	let Some(host) = parents
		.iter_ancestors_inclusive(composer)
		.find(|ancestor| hosts.contains(*ancestor))
	else {
		return;
	};
	commands.entity(composer).insert(RenderSurface(host));
	commands.entity(ev.entity).insert(Focus);
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
/// [`ThreadComposer`] input. Submitting the composer ends the user's turn (the
/// [`UserInput`] action consumes the [`Submit`]), so the thread's `Sequence`
/// advances and the agent answers, no blocking stdin.
pub fn thread_chat_tui(thread: Entity) -> impl Bundle {
	(
		StdioTerminal::default(),
		DoubleBuffer::default(),
		LayoutStyle::flex_col(),
		children![ThreadView::new(thread), ThreadComposer::new(thread)],
	)
}

// ═══════════════════════════════════════════════════════════════════════
// ThreadScenePlugin: load a `.bsx` author scene and mount its UI
// ═══════════════════════════════════════════════════════════════════════

/// Load a `.bsx` author scene on startup and auto-mount a charcell host over each
/// thread it reduces, so an example `main` is just plugins plus this.
///
/// The scene declares its own kick (`{RunThread}`) and store, so no setup-system
/// glue remains: a thread with a `User` actor gets the interactive
/// [`thread_chat_tui`], the rest get the inline [`thread_tui`] transcript.
pub struct ThreadScenePlugin {
	scene: &'static str,
}

impl ThreadScenePlugin {
	/// Run `scene`, the contents of a `.bsx` file (eg `include_str!`).
	pub fn new(scene: &'static str) -> Self { Self { scene } }
}

impl Plugin for ThreadScenePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(ThreadScene(self.scene))
			.add_systems(Startup, spawn_thread_scene)
			.add_systems(Update, mount_thread_ui);
	}
}

#[derive(Resource)]
struct ThreadScene(&'static str);

/// Spawn the author scene; reduction (`First`) turns it into a window + behavior.
fn spawn_thread_scene(world: &mut World) -> Result {
	let scene = world.resource::<ThreadScene>().0;
	BsxTemplate::parse_entry(world, scene)?.spawn(world)?;
	Ok(())
}

/// Mount a charcell host over each freshly-reduced thread: a chat (with composer)
/// when it has a `User` actor, an inline transcript otherwise.
fn mount_thread_ui(
	mut commands: Commands,
	threads: Query<(Entity, &ThreadWindow), Added<ThreadWindow>>,
) {
	for (thread, window) in threads.iter() {
		let has_user = window
			.actors()
			.values()
			.any(|actor| actor.kind() == ActorKind::User);
		match has_user {
			true => commands.spawn(thread_chat_tui(thread)),
			false => commands.spawn(thread_tui(thread)),
		};
	}
}

// ═══════════════════════════════════════════════════════════════════════
// UserInput: the user's turn is a Sequence action
// ═══════════════════════════════════════════════════════════════════════

/// Marks a `User` actor whose turn is to take input from a [`ThreadComposer`].
///
/// Spread it onto a user actor (`<CreateActor name=".." kind="User" {UserInput}/>`);
/// its `on_add` installs the [`user_input_action`]. When the thread's `Sequence`
/// reaches this actor, the action waits for the composer's [`Submit`] (the user
/// pressing Enter ends their turn), appends the typed post, and passes, so the
/// Sequence moves on like any other. A submit outside a user turn installs no
/// observer and is ignored.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = user_input_on_add)]
pub struct UserInput;

fn user_input_on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(Action::<(), Outcome>::new_async(user_input_action));
}

/// The user's turn: await the thread's composer [`Submit`], append the typed post
/// authored by this actor, and [`Pass`] so the `Sequence` advances.
pub async fn user_input_action(cx: ActionContext) -> Result<Outcome> {
	// resolve which thread + actor this user turn belongs to
	let (thread_entity, actor_id, thread_id) = cx
		.caller
		.with_state::<ThreadWindowQuery, _>(
			|entity, window_mut| -> Result<(Entity, ActorId, ThreadId)> {
				Ok((
					window_mut.thread_entity(entity)?,
					window_mut.actor_id(entity)?,
					window_mut.thread_id(entity)?,
				))
			},
		)
		.await??;

	// the composer bound to this thread is the input surface; await its <form>.
	// It may not be mounted yet, so yield a tick and retry until it is.
	let form = loop {
		if let Some(form) = cx
			.caller
			.with_state::<ComposerForms, _>(move |_entity, forms| {
				forms.form_for_thread(thread_entity)
			})
			.await?
		{
			break form;
		}
		cx.caller.with(|_| ()).await?;
	};

	// wait for the user to end their turn: a non-empty composer Submit
	let text = loop {
		let text = cx
			.world()
			.entity(form)
			.await_event::<Submit, _, _, String>(|ev: On<Submit>| {
				ev.values
					.get("message")
					.and_then(|message| message.as_str().ok())
					.unwrap_or_default()
					.to_string()
			})
			.await?;
		if !text.trim().is_empty() {
			break text;
		}
	};

	// append the typed post and advance the Sequence
	cx.caller
		.with_state::<ThreadWindowQuery, _>(
			move |entity, mut window_mut| -> Result {
				window_mut.push_post(
					entity,
					AgentPost::new_text(
						actor_id,
						thread_id,
						text,
						PostStatus::Completed,
					),
				)
			},
		)
		.await??;
	Ok(Pass(()))
}

/// Resolves the `<form>` entity of the [`ThreadComposer`] bound to a thread, so
/// [`user_input_action`] can await its [`Submit`].
#[derive(SystemParam)]
pub struct ComposerForms<'w, 's> {
	composers: Query<'w, 's, (Entity, &'static ThreadComposer)>,
	elements: ElementQuery<'w, 's>,
}

impl ComposerForms<'_, '_> {
	/// The `<form>` entity of the composer bound to `thread`, if one is mounted.
	fn form_for_thread(&self, thread: Entity) -> Option<Entity> {
		let composer = self
			.composers
			.iter()
			.find(|(_, composer)| composer.thread == thread)
			.map(|(entity, _)| entity)?;
		self.elements
			.iter_descendants_inclusive(composer)
			.find(|view| view.tag() == "form")
			.map(|view| view.entity)
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
	/// [`Submit`], consumed by the active [`UserInput`] turn.
	fn content() -> impl Bundle {
		rsx! {
			<form>
				<input name="message" type="text"/>
				<button>"Send"</button>
			</form>
		}
	}
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

	/// The chat layout ([`thread_chat_tui`]'s structure): a [`ThreadView`] and a
	/// [`ThreadComposer`] as siblings under the host still render the transcript,
	/// so the view works as a child alongside the input, not only on the host.
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
	/// full keystroke run is the example's deterministic interaction test.
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
	/// [`focus_chat_composer`] exactly as `thread_chat_tui` wires the real one.
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

		// mount the chat UI on the host (as `thread_chat_tui` does), then kick
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
