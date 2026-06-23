//! The user's input surface: the [`ThreadComposer`] form and the [`UserInput`]
//! Sequence action that consumes its [`Submit`] as the user's turn.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

// ═══════════════════════════════════════════════════════════════════════
// ThreadComposer: agnostic input
// ═══════════════════════════════════════════════════════════════════════

/// An agnostic chat composer: a form whose submit ends the user's turn (consumed
/// by the active [`UserInput`] action, which appends the post). Terminal and web
/// both drive it through `beet_ui`'s form + focus-input machinery, the
/// cross-platform input for a thread (no blocking stdin read).
///
/// Host-agnostic content bound to a thread with an [`OfThread`] relationship. A
/// marker, so the bound thread lives in the relationship, not a stored field. From
/// markup the two spread together onto one entity:
/// `<div {(ThreadComposer, OfThread($thread))}/>`.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = thread_composer_on_add)]
pub struct ThreadComposer;

impl ThreadComposer {
	/// A composer bound to `thread`. Its `<form>` content is attached in `on_add`,
	/// so the bundle works both as a direct spawn and as a markup spread.
	pub fn new(thread: Entity) -> impl Bundle { (Self, OfThread(thread)) }
}

/// Attach the composer's `<form>` (a `message` field + submit button) when added,
/// so the component works as a bare spawn or markup spread. Submitting fires
/// `beet_ui`'s [`Submit`], consumed by the active [`UserInput`] turn.
fn thread_composer_on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).insert(rsx! {
		<form>
			<input name="message" type="text"/>
			<button>"Send"</button>
		</form>
	});
}

/// Clear a [`ThreadComposer`]'s text field after it submits, so the next turn
/// starts from an empty input (the submitted value is already gathered into the
/// [`Submit`], so clearing here never drops it).
pub fn clear_composer_on_submit(
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
	for input in std::iter::once(ev.form)
		.chain(children.iter_descendants(ev.form))
		.filter(|entity| {
			elements
				.get(*entity)
				.map(|element| matches!(element.tag(), "input" | "textarea"))
				.unwrap_or(false)
		}) {
		if let Ok(mut value) = values.get_mut(input) {
			*value = Value::str("");
		}
	}
}

/// When a [`ThreadComposer`]'s text input is added under a charcell host, scope
/// the composer to that host surface and focus the input, so typing and Enter
/// reach it. A directly-spawned charcell host (the local `TuiThreadChat`) skips the
/// router's page-host wiring (`RenderSurface` + focus) that would otherwise do
/// this; the web path keeps its own wiring (no [`DoubleBuffer`] host, so this is
/// a no-op).
pub fn focus_chat_composer(
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
	items: Query<'w, 's, &'static ThreadItems>,
	composers: Query<'w, 's, (), With<ThreadComposer>>,
	elements: ElementQuery<'w, 's>,
}

impl ComposerForms<'_, '_> {
	/// The `<form>` entity of the composer bound to `thread`, if one is mounted:
	/// the thread's first `ThreadItems` member that is a [`ThreadComposer`].
	fn form_for_thread(&self, thread: Entity) -> Option<Entity> {
		let composer = self
			.items
			.get(thread)
			.ok()?
			.iter()
			.find(|item| self.composers.contains(*item))?;
		self.elements
			.iter_descendants_inclusive(composer)
			.find(|view| view.tag() == "form")
			.map(|view| view.entity)
	}
}
