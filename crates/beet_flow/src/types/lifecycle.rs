use crate::prelude::*;
use beet_core::prelude::*;


/// Trait for specifying a 'Run' event, similar to a 'request' in a request-response pattern.
pub trait RunEvent: ActionEvent {
	/// The corresponding 'End' event type
	type End: EndEvent<Run = Self>;
}

/// Trait for specifying an 'End' event, similar to a 'response' in a request-response pattern.
pub trait EndEvent: ActionEvent {
	/// The corresponding 'Run' event type
	type Run: RunEvent<End = Self>;
}

/// Event automatically triggered on the parent of an `event_target` when it triggers an [`End`].
#[derive(Debug, Clone, PartialEq, Eq, ActionEvent)]
pub struct ChildEnd<T>
where
	T: 'static + Send + Sync,
{
	/// The entity that triggered the [`End`]
	child: Entity,
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
	T: Clone + ActionEvent,
{
	/// Trigger [`ChildEnd<T>`] for the *parent* of this event target if it exists,
	/// propagating the [`ActionTrigger::agent`]
	pub fn trigger(mut commands: Commands, ev: &On<T>) {
		let child = ev.action();
		let value = ev.event().clone();
		let agent = ev.agent();

		commands.queue(move |world: &mut World| {
			if let Some(parent) = world.entity(child).get::<ChildOf>().clone() {
				let parent = parent.parent();
				world.entity_mut(parent).trigger_target(
					ChildEnd { child, value }.with_agent(agent),
				);
			}
		})
	}
	/// Get the entity that originated the [`End`]
	pub fn child(&self) -> Entity { self.child }
	/// Get the [`End`] event that the child triggered
	pub fn value(&self) -> &T { &self.value }
	/// Convert a [`ChildEnd`] to an [`End`] by discarding
	/// the `child` field and transfering the `target`
	pub fn inner(self) -> T { self.value }
}



/// This component prevents a [`ChildEnd`] from automatically triggering
/// an [`End`] with the same data, a requirement whenever you want to manually
/// handle propagation, for instance in a [`Sequence`], [`HighestScore`] etc.
#[derive(Component, Reflect)]
pub struct PreventPropagateEnd<T = Outcome> {
	phantom: PhantomData<T>,
}
impl<T> Default for PreventPropagateEnd<T> {
	fn default() -> Self { Self { phantom: default() } }
}

/// Propagate the [`End`] event as a [`ChildEnd`] to this entities
/// parent if it exists.
pub(crate) fn propagate_end<T: ActionEvent>(ev: On<T>, commands: Commands)
where
	T: 'static + Send + Sync + Clone,
{
	ChildEnd::trigger(commands, &ev);
}

/// Propagate the [`ChildEnd`] event as an [`End`] on this entity
/// unless it has a [`PreventPropagateEnd`] component.
pub(crate) fn propagate_child_end<T>(
	mut ev: On<ChildEnd<T>>,
	prevent: Query<(), With<PreventPropagateEnd>>,
) where
	ChildEnd<T>: Clone + ActionEvent,
	T: 'static + Send + Sync + Clone + ActionEvent,
{
	let target = ev.action();
	if !prevent.contains(target) {
		ev.propagate_child();
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[action(run_child)]
	#[derive(Component)]
	struct Parent;

	fn run_child(mut ev: On<GetOutcome>, children: Query<&Children>) {
		let child = children.get(ev.action()).unwrap()[0];
		ev.trigger_action_with_cx(child, GetOutcome);
	}

	#[action(succeed)]
	#[derive(Component)]
	// #[require(PreventPropagateEnd)]
	struct Child;

	fn succeed(ev: On<GetOutcome>, mut commands: Commands) {
		commands
			.entity(ev.action())
			.trigger_target(Outcome::Pass);
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
