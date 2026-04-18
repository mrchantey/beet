//! Extension methods for Bevy's [`EntityWorldMut`].

use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryEntityError;

use crate::prelude::*;


/// Extension trait adding utility methods to [`EntityRef`].
#[extend::ext(name=EntityRefExt)]
pub impl<'a> EntityRef<'a> {
	/// Gets a reference to the component of type `T`, or returns an error if it doesn't exist.
	fn get_or_else<T: Component>(&self) -> Result<&T> {
		self.get::<T>().ok_or_else(|| {
			bevyhow!("Component not found: {}", std::any::type_name::<T>())
		})
	}
}
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

	/// Runs a function with access to a system parameter state.
	fn with_state<T: 'static + SystemParam, O>(
		&mut self,
		func: impl FnOnce(Entity, T::Item<'_, '_>) -> O,
	) -> O {
		let id = self.id();
		self.world_scope(|world| {
			let mut state = world.state::<T>();
			let item = state.get_mut(world);
			let result = func(id, item);
			state.apply(world);
			result
		})
	}

	/// Gets a reference to the component of type `T`, or returns an error if it doesn't exist.
	fn get_or_else<T: Component>(&mut self) -> Result<&T> {
		self.get::<T>().ok_or_else(|| {
			bevyhow!("Component not found: {}", std::any::type_name::<T>())
		})
	}
	/// Gets a mutable reference to the component of type `T`, or returns an error if it doesn't exist.
	fn get_or_else_mut<T: Component<Mutability = Mutable>>(
		&mut self,
	) -> Result<Mut<'_, T>> {
		self.get_mut::<T>().ok_or_else(|| {
			bevyhow!("Component not found: {}", std::any::type_name::<T>())
		})
	}

	/// Runs a function with access to a system parameter state.
	fn with_query<T: 'static + QueryData, O>(
		&mut self,
		func: impl FnOnce(T::Item<'_, '_>) -> O,
	) -> Result<O, QueryEntityError> {
		let id = self.id();
		self.world_scope(|world| {
			let mut state = world.state::<Query<T>>();
			let mut query = state.get_mut(world);
			let result = query.get_mut(id).map(func);
			state.apply(world);
			result
		})
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

	/// Gets a mutable reference to a component, inserting the default value if not present.
	fn get_mut_or_default<T: Component<Mutability = Mutable> + Default>(
		&mut self,
	) -> Mut<'_, T> {
		if !self.contains::<T>() {
			self.insert(T::default());
		}
		self.get_mut::<T>()
			.expect("Component was just inserted or already existed")
	}
}
