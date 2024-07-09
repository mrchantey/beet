use bevy::prelude::*;

/// Used by systems and observers that trigger observers, to specify the target of an insert/change/remove.
#[derive(Debug, Default, Clone, Reflect)]
#[reflect(Default)]
pub enum ComponentTarget {
	#[default]
	This,
	Entity(Entity),
}

impl ComponentTarget {
	pub fn insert(
		&self,
		commands: &mut Commands,
		caller: Entity,
		bundle: impl Bundle,
	) {
		match self {
			Self::This => commands.entity(caller).insert(bundle),
			Self::Entity(entity) => commands.entity(*entity).insert(bundle),
		};
	}
	pub fn remove<T: Bundle>(&self, commands: &mut Commands, caller: Entity) {
		match self {
			Self::This => commands.entity(caller).remove::<T>(),
			Self::Entity(entity) => commands.entity(*entity).remove::<T>(),
		};
	}
}

impl Into<ComponentTarget> for Entity {
	fn into(self) -> ComponentTarget { ComponentTarget::Entity(self) }
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
		let e1 = world.spawn_empty().id();
		let e2 = world.spawn(Name::new("foo")).id();

		let mut commands = world.commands();
		ComponentTarget::This.insert(&mut commands, e1, Name::new("bar"));
		ComponentTarget::Entity(e2).remove::<Name>(&mut commands, e1);
		drop(commands);
		world.flush_commands();
		expect(&world).to_have_component::<Name>(e1)?;
		expect(&world).not().to_have_component::<Name>(e2)?;
		Ok(())
	}
}
