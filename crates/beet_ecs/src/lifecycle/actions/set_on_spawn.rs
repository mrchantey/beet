use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Default, Component, ActionMeta)]
/// Sets a component when this behavior spawns.
/// This does nothing if the entity does not have the component.
pub struct SetOnSpawn<T: GenericActionComponent>(pub T);

impl<T: GenericActionComponent> ActionMeta for SetOnSpawn<T> {
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

impl<T: GenericActionComponent> ActionSystems for SetOnSpawn<T> {
	fn systems() -> SystemConfigs { set_on_spawn::<T>.in_set(PreTickSet) }
}


impl<T: GenericActionComponent> SetOnSpawn<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_spawn<T: GenericActionComponent>(
	mut query: Query<(&SetOnSpawn<T>, &mut T), Added<SetOnSpawn<T>>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<SetOnSpawn<Score>>::default());

		let root = test_constant_behavior_tree(app.world_mut()).value;

		app.world_mut()
			.entity_mut(root)
			.insert(SetOnSpawn(Score::Pass));

		expect(&app).component(root)?.to_be(&Score::Fail)?;
		app.update();
		expect(&app).component(root)?.to_be(&Score::Pass)?;

		Ok(())
	}
}
