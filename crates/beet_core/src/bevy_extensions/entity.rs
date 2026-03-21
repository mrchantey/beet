//! Extension methods for Bevy's [`EntityWorldMut`].

use bevy::ecs::component::Mutable;

use crate::prelude::*;

/// Extension trait adding utility methods to [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutExt)]
pub impl<'a> EntityWorldMut<'a> {
	/// Gets all children as a [`Vec<EntityRef>`].
	fn children(self) -> Vec<EntityRef<'a>> { self.related::<Children>() }

	/// Gets all related entities as a [`Vec<EntityRef>`].
	fn related<R: RelationshipTarget>(self) -> Vec<EntityRef<'a>> {
		let related = self
			.get::<R>()
			.map(|related| related.iter().collect::<Vec<_>>())
			.unwrap_or_default();
		let world = self.into_world_mut();
		related
			.into_iter()
			.map(|child| world.entity(child))
			.collect()
	}

	/// Gets a reference to the component of type `T`, or returns an error if it doesn't exist.
	fn try_get<T: Component>(&self) -> Result<&T> {
		self.get::<T>().ok_or_else(|| {
			bevyhow!("Component not found: {}", std::any::type_name::<T>())
		})
	}

	/// Gets a mutable reference to the child entity at the specified index, if it exists.
	fn child(self, index: usize) -> Option<EntityWorldMut<'a>> {
		let children = self.get::<Children>()?;
		let child_entity = *children.get(index)?;
		let world = self.into_world_mut();
		world.entity_mut(child_entity).xsome()
	}

	/// Logs the names of all components on this entity.
	fn log_component_names(&mut self) -> &mut Self {
		let id = self.id();
		self.world_scope(|world| {
			world.log_component_names(id);
		});
		self
	}

	/// Sets the value of a component if it is different from the existing value, or inserts it if not present.
	fn set_if_ne_or_insert<T: Component<Mutability = Mutable> + PartialEq>(
		&mut self,
		value: T,
	) {
		match self.get_mut::<T>() {
			Some(mut existing) => {
				existing.set_if_neq(value);
			}
			None => {
				self.insert(value);
			}
		}
	}
}
