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
			Self::Entity(entity) => {
				commands.trigger_targets(out, *entity)
			}
			Self::Entities(entities) => {
				commands.trigger_targets(out, entities.clone())
			}
			Self::Global => commands.trigger(out),
		}
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
	fn works() -> Result<()> {
		let mut world = World::new();

		let on_run = observe_triggers::<OnRun>(&mut world);
		let on_result = observe_run_results(&mut world);

		let target = world
			.spawn(EndOnRun::success())
			.id();
		world.spawn((
			TriggerOnRun::new(OnRun).with_target(target),
		)).flush_trigger(OnRun);
		world.flush();

		expect(&on_run).to_have_been_called_times(2)?;
		expect(&on_result).to_have_been_called_times(1)?;

		Ok(())
	}
}
