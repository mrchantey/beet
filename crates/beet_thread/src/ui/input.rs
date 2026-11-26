//! The composer: the [`CreatePostForm`] widget, which posts to its thread the
//! moment it is submitted and stops the thread's turns on request.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;
// styling for the composer: an `inline_class!` per element keeps each rule
// colocated with the widget (a callsite-keyed class per literal).
use beet_ui::prelude::material::colors;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;

/// An agnostic message-entry form posting to its thread: submitting appends
/// the typed post to the thread's records at once, whoever's turn it is, since
/// posting never requires holding a turn. An empty submit is ignored.
///
/// The post's author is the actor named by [`actor`](Self::actor), else the
/// thread's first actor of kind User by id; a thread holding none raises
/// through the error handler. A person's turn ([`UserInput`]) is a wait on the
/// records, so the post passes it whichever window it came from.
///
/// Its stop control, the `Stop` button or Escape in the focused input,
/// triggers [`InterruptTurn`] on every actor of its thread, in every window of
/// the runtime.
///
/// Host-agnostic content bound to a window with an [`OfThread`] relationship.
/// From markup the two spread together onto one entity:
/// `<CreatePostForm {OfThread($thread)}/>`, or
/// `<CreatePostForm actor="ann" {OfThread($thread)}/>` to post as `ann`.
///
/// Surface scoping is the host's job: the charcell host carries
/// `RenderSurface(self)`, so this widget's whole subtree (its `<input>`, included)
/// resolves to it through [`SurfaceQuery`] with no per-widget wiring.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = create_post_form_on_add)]
pub struct CreatePostForm {
	/// The author of its posts, else the first User actor the scene authors.
	pub actor: Option<ActorId>,
}

impl CreatePostForm {
	/// A form bound to `thread`, posting as its first User actor. Its `<form>`
	/// content is attached in `on_add`, so the bundle works both as a direct
	/// spawn and as a markup spread.
	pub fn new(thread: Entity) -> impl Bundle {
		(Self::default(), OfThread(thread))
	}
}

/// Build the widget's `<form>` when a [`CreatePostForm`] is added, so the
/// component works as a bare spawn or markup spread. A plain rust template
/// (`insert(rsx!{..})` merges the `<form>` element onto the entity, so the
/// widget *is* the form).
///
/// `{FocusOnAdd}` gives the input initial focus and `{ClearOnSubmit}` empties
/// it each post. The input surface is resolved from the host (which carries
/// `RenderSurface(self)`), so this hook inserts no surface of its own.
fn create_post_form_on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).insert(rsx! {
		<form {(ClearOnSubmit, form_style())}>
			<input name="message" type="text" {(FocusOnAdd, input_style())}/>
			<button {button_style()}>"Send"</button>
			<button type="button" {(button_style(), OnSpawn::observe(stop_on_click))}>"Stop"</button>
		</form>
	});
}

/// Post the submitted message of a [`CreatePostForm`] to its thread.
pub(super) fn post_on_submit(
	ev: On<Submit>,
	parents: Query<&ChildOf>,
	forms: Query<(&CreatePostForm, &OfThread)>,
	mut threads: Query<(&Thread, &mut ThreadRecords)>,
) -> Result {
	let Some((form, of_thread)) = parents
		.iter_ancestors_inclusive(ev.form)
		.find_map(|entity| forms.get(entity).ok())
	else {
		return Ok(());
	};
	let text = ev
		.values
		.get("message")
		.and_then(|message| message.as_str().ok())
		.unwrap_or_default();
	if text.trim().is_empty() {
		return Ok(());
	}
	let (thread, mut records) =
		threads.get_mut(of_thread.thread()).map_err(|_| {
			bevyhow!(
				"the composer's thread {} has no records to post to, has it been entered?",
				of_thread.thread()
			)
		})?;
	let author = records.author(form.actor.as_ref())?;
	let post =
		AgentPost::new_text(author, thread.id(), text, PostStatus::Completed);
	records.upsert_post(post);
	Ok(())
}

/// The `Stop` button: interrupt every actor of its composer's thread.
fn stop_on_click(
	ev: On<PointerUp>,
	parents: Query<&ChildOf>,
	forms: Query<&OfThread, With<CreatePostForm>>,
	mut stop: StopThread,
) {
	if let Some(of_thread) = parents
		.iter_ancestors_inclusive(ev.event_target())
		.find_map(|entity| forms.get(entity).ok())
	{
		stop.stop(of_thread.thread());
	}
}

/// Escape in a composer's focused input, on the surface it was pressed on:
/// interrupt every actor of its thread.
pub(super) fn stop_on_escape(
	mut keys: MessageReader<KeyboardInput>,
	focused: Query<Entity, With<Focus>>,
	parents: Query<&ChildOf>,
	forms: Query<&OfThread, With<CreatePostForm>>,
	surfaces: SurfaceQuery,
	mut stop: StopThread,
) {
	let escaped = keys
		.read()
		.filter(|key| {
			key.state == ButtonState::Pressed && key.logical_key == Key::Escape
		})
		.map(|key| key.window)
		.collect::<HashSet<_>>();
	for target in focused.iter().filter(|target| {
		escaped
			.iter()
			.any(|window| surfaces.matches(*target, *window))
	}) {
		if let Some(of_thread) = parents
			.iter_ancestors_inclusive(target)
			.find_map(|entity| forms.get(entity).ok())
		{
			stop.stop(of_thread.thread());
		}
	}
}

/// Interrupts every actor of a stored thread, in each of its windows.
#[derive(SystemParam)]
pub(super) struct StopThread<'w, 's> {
	threads: Query<'w, 's, &'static Thread>,
	windows: ThreadWindows<'w, 's>,
	commands: Commands<'w, 's>,
}

impl StopThread<'_, '_> {
	/// Trigger [`InterruptTurn`] on every actor of the stored thread the
	/// window `window` is open onto.
	fn stop(&mut self, window: Entity) {
		let Ok(thread) = self.threads.get(window) else {
			return;
		};
		for actor in self.windows.actors(thread.id()).collect::<Vec<_>>() {
			self.commands.trigger(InterruptTurn::new(actor));
		}
	}
}

/// The composer row: a stretched horizontal flex, gapped, separated from the
/// transcript by a top border.
fn form_style() -> OnSpawn {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Horizontal
		),
		(
			style::common_props::AlignItemsProp,
			style::AlignItems::Stretch
		),
		(style::common_props::ColumnGapProp, style::Length::Rem(1.)),
		(style::common_props::BorderTopWidth, style::Length::Rem(1.)),
		Declaration::token(
			style::common_props::BorderColorProp,
			colors::OutlineVariant
		),
	]
}

/// The message input: grows to fill the row, on the raised surface tint.
fn input_style() -> OnSpawn {
	inline_class![
		(style::common_props::FlexGrowProp, 1_f32),
		Declaration::token(
			style::common_props::BackgroundColor,
			colors::SurfaceContainerHigh
		),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::OnSurface
		),
	]
}

/// The send button: bold on the primary tint.
fn button_style() -> OnSpawn {
	inline_class![
		(style::common_props::FontWeightProp, style::FontWeight::Bold),
		Declaration::token(
			style::common_props::BackgroundColor,
			colors::Primary
		),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::OnPrimary
		),
	]
}
