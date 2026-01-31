//! Lifecycle traits and components for action event propagation.
//!
//! This module defines the request/response pattern used by actions through
//! [`RunEvent`] and [`EndEvent`] traits, along with automatic event propagation
//! via [`ChildEnd`].
use crate::prelude::*;
use beet_core::prelude::*;


/// Trait for "Run" events that request an action to execute.
///
/// Run events follow a request/response pattern similar to HTTP:
/// - A [`RunEvent`] is triggered on an action (the "request")
/// - The action eventually responds with its associated [`EndEvent`]
///
/// Events implementing this trait use [`EntityTargetEvent`] as their trigger type.
///
/// # Example
///
/// The built-in [`GetOutcome`] is the most common run event:
///
/// ```
/// # use beet_flow::prelude::*;
/// # use beet_core::prelude::*;
/// # let mut world = World::new();
/// // Request an action to run and return an Outcome
/// world
///     .spawn(EndWith(Outcome::Pass))
///     .trigger_target(GetOutcome);
/// ```
pub trait RunEvent: EntityTargetEvent {
	/// The corresponding "End" event type returned by actions.
	type End: EndEvent<Run = Self>;
}

/// Trait for "End" events that signal action completion.
///
/// End events are responses to [`RunEvent`] requests, indicating that an
/// action has finished executing. The event payload contains the result.
///
/// Events implementing this trait use [`EntityTargetEvent`] as their trigger type
/// and must be [`Clone`] for propagation.
///
/// # Propagation
///
/// When an action triggers its end event, the event automatically propagates
/// up the hierarchy as [`ChildEnd<T>`] unless blocked by [`PreventPropagateEnd`].
pub trait EndEvent: EntityTargetEvent + Clone {
	/// The corresponding "Run" event type that initiated this response.
	type Run: RunEvent<End = Self>;
}

/// Event triggered on a parent when a child action ends.
///
/// When an action triggers an [`EndEvent`], this wrapper is automatically
/// sent to its parent entity (if one exists). This enables composite actions
/// like [`Sequence`] and [`Parallel`] to coordinate child behavior.
///
/// # Automatic Propagation
///
/// By default, receiving a `ChildEnd` will trigger the inner event on the
/// parent entity, continuing propagation up the tree. Use [`PreventPropagateEnd`]
/// to intercept and handle child results manually.
#[derive(Debug, Clone, PartialEq, Eq, EntityTargetEvent)]
pub struct ChildEnd<T>
where
	T: 'static + Send + Sync,
{
	/// The entity that triggered the end event.
	child: Entity,
	/// The end event payload from the child.
	value: T,
}

impl<T> std::ops::Deref for ChildEnd<T>
where
	T: 'static + Send + Sync,
{
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.value }
}
impl<T> std::ops::DerefMut for ChildEnd<T>
where
	T: 'static + Send + Sync,
{
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<T> ChildEnd<T>
where
	T: Clone + Event,
{
	/// Triggers [`ChildEnd<T>`] on the parent of the given entity if it exists.
	pub fn trigger(mut commands: Commands, child: Entity, value: T) {
		commands.queue(move |world: &mut World| {
			if let Some(parent) = world.entity(child).get::<ChildOf>().clone() {
				let parent = parent.parent();
				world
					.entity_mut(parent)
					.trigger_target(ChildEnd { child, value });
			}
		})
	}
	/// Returns the entity that originated the end event.
	pub fn child(&self) -> Entity { self.child }
	/// Returns a reference to the end event payload.
	pub fn value(&self) -> &T { &self.value }
	/// Consumes this wrapper and returns the inner end event.
	pub fn inner(self) -> T { self.value }
}



/// Prevents automatic propagation of [`ChildEnd`] events.
///
/// When attached to an entity, this component stops [`ChildEnd<T>`] from
/// automatically triggering the inner `T` event on that entity. This is
/// required for composite actions that need to manually handle child results.
///
/// # Usage
///
/// Actions like [`Sequence`], [`Parallel`], and [`HighestScore`] use this
/// component to intercept child results and implement their control flow logic.
///
/// ```
/// # use beet_flow::prelude::*;
/// # use beet_core::prelude::*;
/// # let mut world = World::new();
/// // This entity will NOT automatically propagate child outcomes
/// world.spawn(PreventPropagateEnd::<Outcome>::default());
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct PreventPropagateEnd<T = Outcome> {
	phantom: PhantomData<T>,
}
impl<T> Default for PreventPropagateEnd<T> {
	fn default() -> Self { Self { phantom: default() } }
}

/// Propagates an [`EndEvent`] to the parent as a [`ChildEnd`].
pub(crate) fn propagate_end<T: EndEvent>(ev: On<T>, commands: Commands) {
	ChildEnd::trigger(commands, ev.target(), ev.event().clone());
}

/// Propagates [`ChildEnd`] as the inner event unless [`PreventPropagateEnd`] is present.
pub(crate) fn propagate_child_end<T>(
	ev: On<ChildEnd<T>>,
	mut commands: Commands,
	prevent: Query<(), With<PreventPropagateEnd>>,
) where
	T: EntityTargetEvent + Clone,
{
	let target = ev.target();
	if !prevent.contains(target) {
		let value = ev.event().clone().inner();
		commands.entity(target).trigger_target(value);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[action(run_child)]
	#[derive(Component)]
	struct Parent;

	fn run_child(
		ev: On<GetOutcome>,
		children: Query<&Children>,
		mut commands: Commands,
	) {
		let child = children.get(ev.target()).unwrap()[0];
		commands.entity(child).trigger_target(GetOutcome);
	}

	#[action(succeed)]
	#[derive(Component)]
	struct Child;

	fn succeed(ev: On<GetOutcome>, mut commands: Commands) {
		commands.entity(ev.target()).trigger_target(Outcome::Pass);
	}

	#[test]
	fn works() {
		let mut world = ControlFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, ExitOnEnd, children![Child]))
			.trigger_target(GetOutcome)
			.flush();
		world.should_exit().xpect_eq(Some(AppExit::Success));
	}
	#[test]
	fn prevent_propagate() {
		let mut world = ControlFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((
				Parent,
				PreventPropagateEnd::<Outcome>::default(),
				children![(Child)],
			))
			.trigger_target(GetOutcome)
			.flush();
		world.should_exit().xpect_none();
	}
}
