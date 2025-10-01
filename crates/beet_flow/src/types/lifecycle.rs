use beet_core::prelude::*;

/// Alias for `Run::<()>(())`
pub const RUN: Run<()> = Run(());

#[derive(EntityTargetEvent)]
pub struct Run<T: 'static + Send + Sync = ()>(pub T);
impl<T> Default for Run<T>
where
	T: 'static + Send + Sync + Default,
{
	fn default() -> Self { Self(default()) }
}


pub type EndResult = Result<(), ()>;

#[derive(Debug, Clone, PartialEq, Eq, EntityTargetEvent)]
pub struct End<T = EndResult>
where
	T: 'static + Send + Sync,
{
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

impl<T> Default for End<T>
where
	T: 'static + Send + Sync + Default,
{
	fn default() -> Self { Self::new(Default::default()) }
}
impl<T> End<T>
where
	T: 'static + Send + Sync,
{
	pub fn new(value: T) -> Self { Self { value } }

	pub fn into_child_end(self) -> ChildEnd<T> {
		ChildEnd { value: self.value }
	}
}

impl End<EndResult> {
	pub fn success() -> Self { Self::new(Ok(())) }
	pub fn failure() -> Self { Self::new(Err(())) }
}

#[derive(Debug, Clone, PartialEq, Eq, EntityTargetEvent)]
pub struct ChildEnd<T = EndResult>
where
	T: 'static + Send + Sync,
{
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
	pub fn into_end(self) -> End<T> { End { value: self.value } }
}

/// Add this to an entity to prevent the run result from bubbling up.
/// Any action that requires this needs to manually call OnChildResult
/// on the parent entity. For an example, see [`Repeat`].
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
	if let Ok(parent) = parents.get(ev.target()) {
		commands
			.entity(parent.parent())
			.trigger_target(ev.event().clone().into_child_end());
	}
}

/// Propagate the [`ChildEnd`] event as an [`End`] on this entity
/// unless it has a [`PreventEndPropagate`] component.
pub(crate) fn propagate_child_end<T>(
	ev: On<ChildEnd<T>>,
	mut commands: Commands,
	prevent: Query<(), With<PreventPropagateEnd>>,
) where
	T: 'static + Send + Sync + Clone,
{
	let target = ev.target();
	if !prevent.contains(target) {
		commands
			.entity(target)
			.trigger_target(ev.event().clone().into_end());
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
		let child = children.get(ev.trigger().event_target()).unwrap()[0];
		commands.entity(child).trigger_target(RUN);
	}

	fn exit_on_result(
		ev: On<End>,
		mut commands: Commands,
		// children: Query<&Children>,
	) {
		ev.event().xpect_eq(End::success());
		// let child = children.get(ev.trigger().event_target()).unwrap()[0];
		// ev.trigger().event_target().xpect_eq(child);
		commands.write_message(AppExit::Success);
	}

	#[action(succeed)]
	#[derive(Component)]
	// #[require(PreventEndPropagate)]
	struct Child;

	fn succeed(ev: On<Run>, mut commands: Commands) {
		commands
			.entity(ev.trigger().event_target())
			.trigger_target(End::success());
	}

	#[test]
	fn works() {
		let mut world = BeetFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world.spawn((Parent, children![Child])).trigger_target(RUN);
		world.should_exit().xpect_eq(Some(AppExit::Success));
	}
	#[test]
	fn prevent_propagate() {
		let mut world = BeetFlowPlugin::world();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((Parent, PreventPropagateEnd, children![(Child)]))
			.trigger_target(RUN);
		world.should_exit().xpect_none();
	}
}
