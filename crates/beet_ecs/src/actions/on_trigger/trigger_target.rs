use bevy::prelude::*;


/// Used by systems and observers that trigger observers, to specify the target of the trigger.
#[derive(Debug, Default, Clone, Reflect)]
#[reflect(Default)]
pub enum TriggerTarget {
	#[default]
	This,
	Entity(Entity),
	Entities(Vec<Entity>),
	Global,
}

impl TriggerTarget {
	pub fn trigger(
		&self,
		commands: &mut Commands,
		caller: Entity,
		out: impl Event,
	) {
		match self {
			Self::This => commands.trigger_targets(out, caller),
			Self::Entity(entity) => commands.trigger_targets(out, *entity),
			Self::Entities(entities) => {
				commands.trigger_targets(out, entities.clone())
			}
			Self::Global => commands.trigger(out),
		}
	}
	/// # Panics
	/// If the target is `Global`
	pub fn insert(
		&self,
		commands: &mut Commands,
		caller: Entity,
		bundle: impl Bundle + Clone,
	) {
		match self {
			Self::This => {
				commands.entity(caller).insert(bundle);
			}
			Self::Entity(entity) => {
				commands.entity(*entity).insert(bundle);
			}
			Self::Entities(entities) => {
				for entity in entities.iter() {
					commands.entity(*entity).insert(bundle.clone());
				}
			}
			Self::Global => panic!("Cannot insert to global target"),
		};
	}
	/// # Panics
	/// If the target is `Global`
	pub fn remove<T: Bundle>(&self, commands: &mut Commands, caller: Entity) {
		match self {
			Self::This => {
				commands.entity(caller).remove::<T>();
			}
			Self::Entity(entity) => {
				commands.entity(*entity).remove::<T>();
			}
			Self::Entities(entities) => {
				for entity in entities.iter() {
					commands.entity(*entity).remove::<T>();
				}
			}
			Self::Global => panic!("Cannot remove from global target"),
		};
	}
}

impl Into<TriggerTarget> for Entity {
	fn into(self) -> TriggerTarget { TriggerTarget::Entity(self) }
}
impl Into<TriggerTarget> for Vec<Entity> {
	fn into(self) -> TriggerTarget { TriggerTarget::Entities(self) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn trigger() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<(TriggerOnRun<OnRun>, EndOnRun)>::default(),
		);
		let world = app.world_mut();

		let on_run = observe_triggers::<OnRun>(world);
		let on_result = observe_run_results(world);

		let target = world.spawn(EndOnRun::success()).id();
		world
			.spawn((TriggerOnRun::new(OnRun).with_target(target),))
			.flush_trigger(OnRun);
		world.flush();

		expect(&on_run).to_have_been_called_times(2)?;
		expect(&on_result).to_have_been_called_times(1)?;

		Ok(())
	}

	#[test]
	fn insert_remove() -> Result<()> {
		let mut world = World::new();
		let e1 = world.spawn_empty().id();
		let e2 = world.spawn(Name::new("foo")).id();

		let mut commands = world.commands();
		TriggerTarget::This.insert(&mut commands, e1, Name::new("bar"));
		TriggerTarget::Entity(e2).remove::<Name>(&mut commands, e1);
		drop(commands);
		world.flush_commands();
		expect(&world).to_have_component::<Name>(e1)?;
		expect(&world).not().to_have_component::<Name>(e2)?;
		Ok(())
	}
}
