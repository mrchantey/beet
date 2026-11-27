//! The composer: the [`CreatePostForm`] widget, which posts to its thread the
//! moment it is submitted, edits and deletes its actor's own posts, and stops
//! the thread's turns on request.

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
/// A [`ThreadView`] beside it on its page, bound to the same window, is bound
/// to it ([`ViewComposer`]): each post of the composer's actor that it may
/// change carries `edit` and `delete`. `edit` hands the post's text to the
/// composer, whose next submit edits that post ([`EditingPost`],
/// [`EditPost`]) and whose Escape leaves the edit rather than stopping;
/// `delete` deletes it ([`DeletePost`]). A refused or conflicting change
/// raises through the error handler.
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

/// The post a [`CreatePostForm`] is editing, its text handed to the
/// composer's input: the next submit edits that post rather than posting,
/// and Escape leaves the edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct EditingPost(pub PostId);

/// The composer a [`ThreadView`] is bound to: a [`CreatePostForm`] beside it
/// on its page, bound to the same window.
#[derive(SystemParam)]
pub struct ViewComposer<'w, 's> {
	parents: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
	forms: Query<'w, 's, (&'static CreatePostForm, &'static OfThread)>,
	views: Query<'w, 's, &'static OfThread, With<ThreadView>>,
	rows: Query<'w, 's, &'static ThreadRow>,
	elements: Query<'w, 's, &'static Element>,
}

impl ViewComposer<'_, '_> {
	/// The composer of the view `view` of the window `window`, if its page
	/// holds one.
	pub fn of(&self, view: Entity, window: Entity) -> Option<&CreatePostForm> {
		self.form_of(view, window)
			.and_then(|form| self.forms.get(form).ok())
			.map(|(form, _)| form)
	}

	/// The entity of the composer of the view `view` of the window `window`.
	fn form_of(&self, view: Entity, window: Entity) -> Option<Entity> {
		let parent = self.parents.get(view).ok()?.parent();
		self.children.get(parent).ok()?.iter().find(|sibling| {
			self.forms
				.get(*sibling)
				.is_ok_and(|(_, of_thread)| of_thread.thread() == window)
		})
	}

	/// The post of the row holding `control`, the window its view shows and
	/// the entity of the view's composer, for a row's control acting on it.
	fn control(&self, control: Entity) -> Option<(PostId, Entity, Entity)> {
		let post = self
			.parents
			.iter_ancestors_inclusive(control)
			.find_map(|entity| self.rows.get(entity).ok())?
			.0;
		let (view, window) = self
			.parents
			.iter_ancestors_inclusive(control)
			.find_map(|entity| {
				self.views
					.get(entity)
					.ok()
					.map(|of_thread| (entity, of_thread.thread()))
			})?;
		(post, window, self.form_of(view, window)?).xsome()
	}

	/// The `<input>` of the composer `form`.
	fn input(&self, form: Entity) -> Option<Entity> {
		self.children.iter_descendants(form).find(|entity| {
			self.elements
				.get(*entity)
				.is_ok_and(|element| element.tag() == "input")
		})
	}
}

/// A row's `edit`: hand the post's text to the view's composer, focused,
/// its next submit editing the post.
pub(crate) fn edit_on_click(
	ev: On<PointerUp>,
	composers: ViewComposer,
	records: Query<&ThreadRecords>,
	mut commands: Commands,
) {
	let Some((post, window, form)) = composers.control(ev.event_target())
	else {
		return;
	};
	let text = records
		.get(window)
		.ok()
		.and_then(|records| records.post(post))
		.and_then(|post| post.text())
		.unwrap_or_default()
		.to_string();
	commands.entity(form).insert(EditingPost(post));
	if let Some(input) = composers.input(form) {
		commands.entity(input).insert((Value::str(text), Focus));
	}
}

/// A row's `delete`: delete the post as the view's composer's actor.
pub(crate) fn delete_on_click(
	ev: On<PointerUp>,
	composers: ViewComposer,
	records: Query<&ThreadRecords>,
	commands: AsyncCommands,
) -> Result {
	let Some((post, window, form)) = composers.control(ev.event_target())
	else {
		return Ok(());
	};
	let actor = records
		.get(window)?
		.author(composers.forms.get(form)?.0.actor.as_ref())?;
	commands.run(async move |world: AsyncWorld| -> Result {
		DeletePost::new(actor, post)
			.apply(&world.entity(window))
			.await
	});
	Ok(())
}

/// Post the submitted message of a [`CreatePostForm`] to its thread, or,
/// while it is editing a post ([`EditingPost`]), edit that post.
pub(super) fn post_on_submit(
	ev: On<Submit>,
	parents: Query<&ChildOf>,
	forms: Query<(Entity, &CreatePostForm, &OfThread, Option<&EditingPost>)>,
	mut threads: Query<(&Thread, &mut ThreadRecords)>,
	mut commands: AsyncCommands,
) -> Result {
	let Some((entity, form, of_thread, editing)) = parents
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
	if let Some(EditingPost(post)) = editing.copied() {
		let (window, text) = (of_thread.thread(), text.to_string());
		commands.entity(entity).remove::<EditingPost>();
		commands.run(async move |world: AsyncWorld| -> Result {
			EditPost::new(author, post, text)
				.apply(&world.entity(window))
				.await
		});
		return Ok(());
	}
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
/// leave the post it is editing, its input emptied, else interrupt every
/// actor of its thread.
pub(super) fn stop_on_escape(
	mut keys: MessageReader<KeyboardInput>,
	focused: Query<Entity, With<Focus>>,
	parents: Query<&ChildOf>,
	forms: Query<(Entity, &OfThread, Has<EditingPost>), With<CreatePostForm>>,
	composers: ViewComposer,
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
		match parents
			.iter_ancestors_inclusive(target)
			.find_map(|entity| forms.get(entity).ok())
		{
			Some((form, _, true)) => {
				stop.commands.entity(form).remove::<EditingPost>();
				if let Some(input) = composers.input(form) {
					stop.commands.entity(input).insert(Value::str(""));
				}
			}
			Some((_, of_thread, false)) => stop.stop(of_thread.thread()),
			None => {}
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
