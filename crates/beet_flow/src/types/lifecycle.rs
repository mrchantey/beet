use crate::prelude::*;
use beet_core::prelude::*;

pub trait RunPayload: EventPayload {
	type End: EndPayload<Run = Self>;
}

pub trait EndPayload: EventPayload {
	type Run: RunPayload<End = Self>;
}

/// An [`EntityEvent`] requesting this entity to trigger a corresponding
/// [`End`] event.
/// The event pair is defined as [`RunPayload::End`] and [`EndPayload::Run`]
/// The default pair is [`GetOutcome`]/[`Outcome`] but the mechanism is general-purpose in nature,
/// for instance it can als be used for a utility ai [`GetScore`]/[`Score`] pair.
#[derive(Debug, Clone, EntityEvent)]
pub struct Run<T = GetOutcome> {
	#[event_target]
	target: Entity,
	value: T,
}
impl<T> From<Entity> for Run<T>
where
	T: Default,
{
	fn from(target: Entity) -> Self { Self::new(target, default()) }
}
impl<T> Run<T> {
	pub fn new(target: Entity, value: T) -> Self { Self { target, value } }
	pub fn target(&self) -> Entity { self.target }
	pub fn value(&self) -> &T { &self.value }
}


#[derive(Debug, Clone, PartialEq, Eq, EntityEvent)]
pub struct End<T = Outcome> {
	#[event_target]
	target: Entity,
	value: T,
}

impl<T> std::ops::Deref for End<T>
where
	T: 'static + Send + Sync,
{
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.value }
}
impl<T> std::ops::DerefMut for End<T>
where
	T: 'static + Send + Sync,
{
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<T> From<Entity> for End<T>
where
	T: 'static + Send + Sync + Default,
{
	fn from(value: Entity) -> Self { Self::new(value, default()) }
}

impl<T> End<T>
where
	T: 'static + Send + Sync,
{
	pub fn new(target: Entity, value: T) -> Self { Self { target, value } }
	pub fn target(&self) -> Entity { self.target }
	pub fn value(&self) -> &T { &self.value }

	pub fn into_child_end(self, target: Entity) -> ChildEnd<T> {
		ChildEnd {
			target,
			child: self.target,
			value: self.value,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, EntityEvent)]
pub struct ChildEnd<T = Outcome>
where
	T: 'static + Send + Sync,
{
	/// The parent that this event is being triggered on
	#[event_target]
	target: Entity,
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
	T: 'static + Send + Sync,
{
	pub fn target(&self) -> Entity { self.target }
	pub fn child(&self) -> Entity { self.child }
	pub fn value(&self) -> &T { &self.value }
	/// Convert a [`ChildEnd`] to an [`End`] by discarding
	/// the `child` field and transfering the `target`
	pub fn into_end(self) -> End<T> {
		End {
			value: self.value,
			target: self.target,
		}
	}
}

pub const PREVENT_PROPAGATE_END: PreventPropagateEnd = PreventPropagateEnd {
	phantom: PhantomData,
};

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
pub(crate) fn propagate_end<T>(
	ev: On<End<T>>,
	mut commands: Commands,
	parents: Query<&ChildOf>,
) where
	T: 'static + Send + Sync + Clone,
{
	if let Ok(parent) = parents.get(ev.event_target()) {
		commands.trigger(ev.event().clone().into_child_end(parent.parent()));
	}
}

/// Propagate the [`ChildEnd`] event as an [`End`] on this entity
/// unless it has a [`PreventPropagateEnd`] component.
pub(crate) fn propagate_child_end<T>(
	ev: On<ChildEnd<T>>,
	mut commands: Commands,
	prevent: Query<(), With<PreventPropagateEnd>>,
) where
	T: 'static + Send + Sync + Clone,
{
	let target = ev.event_target();
	if !prevent.contains(target) {
		commands.trigger(ev.event().clone().into_end());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[action(run_child, exit_on_result)]
	#[derive(Component)]
	struct Parent;

	fn run_child(
		ev: On<Run>,
		mut commands: Commands,
		children: Query<&Children>,
	) {
		let child = children.get(ev.event_target()).unwrap()[0];
		commands.entity(child).trigger_payload(GetOutcome);
	}

	fn exit_on_result(
		ev: On<End>,
		mut commands: Commands,
		// children: Query<&Children>,
	) {
		ev.event().value.is_pass().xpect_true();
		commands.write_message(AppExit::Success);
	}

	#[action(succeed)]
	#[derive(Component)]
	// #[require(PreventPropagateEnd)]
	struct Child;

	fn succeed(ev: On<Run>, mut commands: Commands) {
		commands.entity(ev.event_target()).trigger_payload(Outcome::Pass);
	}

	#[test]
	fn works() {
		let mut world = BeetFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, children![Child]))
			.trigger_payload(GetOutcome)
			.flush();
		world.should_exit().xpect_eq(Some(AppExit::Success));
	}
	#[test]
	fn prevent_propagate() {
		let mut world = BeetFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, PREVENT_PROPAGATE_END, children![(Child)]))
			.trigger_payload(GetOutcome)
			.flush();
		world.should_exit().xpect_none();
	}
}
