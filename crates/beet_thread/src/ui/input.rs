//! The user's input surface: the [`CreatePostForm`] widget and the [`UserInput`]
//! Sequence action that consumes its [`Submit`] as the user's turn.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

// ═══════════════════════════════════════════════════════════════════════
// CreatePostForm: agnostic input
// ═══════════════════════════════════════════════════════════════════════

/// An agnostic message-entry form: a `<form>` whose submit ends the user's turn
/// (consumed by the active [`UserInput`] action, which appends the post).
/// Terminal and web both drive it through `beet_ui`'s form + focus-input
/// machinery, the cross-platform input for a thread (no blocking stdin read).
///
/// Host-agnostic content bound to a thread with an [`OfThread`] relationship. A
/// marker, so the bound thread lives in the relationship, not a stored field. From
/// markup the two spread together onto one entity:
/// `<div {(CreatePostForm, OfThread($thread))}/>`.
///
/// Surface scoping is the host's job: the charcell host carries
/// `RenderSurface(self)`, so this widget's whole subtree (its `<input>`, included)
/// resolves to it through [`SurfaceQuery`] with no per-widget wiring.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = create_post_form_on_add)]
pub struct CreatePostForm;

impl CreatePostForm {
	/// A form bound to `thread`. Its `<form>` content is attached in `on_add`,
	/// so the bundle works both as a direct spawn and as a markup spread.
	pub fn new(thread: Entity) -> impl Bundle { (Self, OfThread(thread)) }
}

/// The crate-shipped `CreatePostForm.bsx` source, embedded at compile time so a
/// deployed binary (with no source tree on disk) still carries the bytes. The
/// bytes are seeded into an in-memory [`BlobStore`] and registered through the
/// same store-backed path as site templates (see [`ThreadUiPlugin`]); a site
/// shipping its own `CreatePostForm.bsx` overrides it.
pub const CREATE_POST_FORM_BSX: &str = include_str!("CreatePostForm.bsx");

/// The name [`CreatePostForm`]'s `.bsx` template is registered under, the module
/// path derived from its store key `templates/CreatePostForm.bsx`.
pub const CREATE_POST_FORM_TEMPLATE: &str = "CreatePostForm";

/// Build the widget's `<form>` from the [`BsxTemplateRegistry`]-registered
/// `CreatePostForm.bsx`, so the component works as a bare spawn or markup spread.
/// [`ThreadUiPlugin`] registers that template through an embedded blob store
/// *asynchronously* at [`Startup`], so a composer spawned by a booting scene can
/// run before it settles. Rather than fail, this awaits the registration (a
/// queued async task that yields until the template lands), then builds the form;
/// a site shipping its own `CreatePostForm.bsx` registers under the same path and
/// is picked up here. Submitting fires `beet_ui`'s [`Submit`], consumed by the
/// active [`UserInput`] turn; `{FocusOnAdd}` on the form's `<input>` (in the
/// `.bsx`) gives it initial focus. The input surface is resolved from the host
/// (which carries `RenderSurface(self)`), so this hook inserts no surface of its own.
fn create_post_form_on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).queue_async_local(
		move |entity: AsyncEntity| async move {
			// await the (async) template registration, yielding a tick between polls
			let registry = loop {
				if let Some(registry) = entity
					.world()
					.with(|world: &mut World| {
						world
							.get_resource::<BsxTemplateRegistry>()
							.filter(|registry| {
								registry.contains(CREATE_POST_FORM_TEMPLATE)
							})
							.cloned()
					})
					.await
				{
					break registry;
				}
				entity.with(|_| ()).await?;
			};
			let nodes = registry
				.get(CREATE_POST_FORM_TEMPLATE)
				.ok_or_else(|| {
					bevyhow!(
						"no BSX template registered under `{CREATE_POST_FORM_TEMPLATE}`"
					)
				})?
				.nodes
				.clone();
			entity
				.with(move |mut entity| -> Result {
					entity.insert_template(BsxTemplate::new(nodes, registry))?;
					Ok(())
				})
				.await??;
			Ok(())
		},
	);
}

// ═══════════════════════════════════════════════════════════════════════
// UserInput: the user's turn is a Sequence action
// ═══════════════════════════════════════════════════════════════════════

/// Marks a `User` actor whose turn is to take input from a [`CreatePostForm`].
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

/// Resolves the `<form>` entity of the [`CreatePostForm`] bound to a thread, so
/// [`user_input_action`] can await its [`Submit`].
#[derive(SystemParam)]
pub struct ComposerForms<'w, 's> {
	items: Query<'w, 's, &'static ThreadItems>,
	forms: Query<'w, 's, (), With<CreatePostForm>>,
	elements: ElementQuery<'w, 's>,
}

impl ComposerForms<'_, '_> {
	/// The `<form>` entity of the widget bound to `thread`, if one is mounted:
	/// the thread's first `ThreadItems` member that is a [`CreatePostForm`].
	fn form_for_thread(&self, thread: Entity) -> Option<Entity> {
		let widget = self
			.items
			.get(thread)
			.ok()?
			.iter()
			.find(|item| self.forms.contains(*item))?;
		self.elements
			.iter_descendants_inclusive(widget)
			.find(|view| view.tag() == "form")
			.map(|view| view.entity)
	}
}
