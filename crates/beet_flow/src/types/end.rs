use beet_core::prelude::*;



/// The most common End payload, a [`Result<(),()>`] used to indicate run status
pub type EndResult = Result<(), ()>;

#[derive(Debug, Clone, PartialEq, Eq, EntityEvent)]
pub struct End<T = EndResult> {
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

impl End<EndResult> {}


#[derive(Debug, Default, Clone)]
pub struct IntoEnd<T = EndResult>(T);
impl<T> IntoEnd<T> {
	pub fn new(value: T) -> Self { Self(value) }
}
impl IntoEnd<EndResult> {
	pub fn success() -> Self { Self(Ok(())) }
	pub fn failure() -> Self { Self(Err(())) }
}

impl<T: 'static + Send + Sync> IntoEntityEvent for IntoEnd<T> {
	type Event = End<T>;
	fn into_entity_event(self, entity: Entity) -> Self::Event {
		End::new(entity, self.0)
	}
}


#[derive(Debug, Clone, PartialEq, Eq, EntityEvent)]
pub struct ChildEnd<T = EndResult>
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
	/// Convert a [`ChildEnd`] to an [`End`] by discarding
	/// the `child` field and transfering the `target`
	pub fn into_end(self) -> End<T> {
		End {
			value: self.value,
			target: self.target,
		}
	}
}

/// This component prevents a [`ChildEnd`] from automatically triggering
/// an [`End`] with the same data, a requirement whenever you want to manually
/// handle propagation, for instance in a [`Sequence`], [`HighestScore`] etc.
#[derive(Default, Component, Reflect)]
pub struct PreventPropagateEnd;

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
		commands.entity(child).trigger_entity(RUN);
	}

	fn exit_on_result(
		ev: On<End>,
		mut commands: Commands,
		// children: Query<&Children>,
	) {
		ev.event().value.xpect_ok();
		// let child = children.get(ev.trigger().event_target()).unwrap()[0];
		// ev.trigger().event_target().xpect_eq(child);
		commands.write_message(AppExit::Success);
	}

	#[action(succeed)]
	#[derive(Component)]
	// #[require(PreventPropagateEnd)]
	struct Child;

	fn succeed(ev: On<Run>, mut commands: Commands) {
		commands
			.entity(ev.event_target())
			.trigger_entity(IntoEnd::success());
	}

	#[test]
	fn works() {
		let mut world = BeetFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, children![Child]))
			.trigger_entity(RUN)
			.flush();
		world.should_exit().xpect_eq(Some(AppExit::Success));
	}
	#[test]
	fn prevent_propagate() {
		let mut world = BeetFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, PreventPropagateEnd, children![(Child)]))
			.trigger_entity(RUN)
			.flush();
		world.should_exit().xpect_none();
	}
}
