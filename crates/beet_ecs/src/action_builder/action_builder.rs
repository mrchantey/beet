// use crate::action::ActionCategory;
// use bevy::ecs::component::ComponentId;
// use bevy::ecs::world::DeferredWorld;
// use bevy::prelude::*;
// use bevy::reflect::GetTypeRegistration;
// use std::marker::PhantomData;


// pub struct ActionBuilder<T: ActionBuilderTypes> {
// 	category: ActionCategory,
// 	on_add: Option<fn(DeferredWorld, Entity, ComponentId)>,
// 	on_remove: Option<fn(DeferredWorld, Entity, ComponentId)>,
// 	// observers: T::Observers,
// 	phantom: PhantomData<T>,
// }

// impl<T: ActionBuilderComponent> Default
// 	for ActionBuilder<DefaultActionTypes<T>>
// {
// 	fn default() -> Self {
// 		Self {
// 			category: ActionCategory::default(),
// 			on_add: None,
// 			on_remove: None,
// 			phantom: PhantomData,
// 		}
// 	}
// }

// // just use name as a placeholder
// impl ActionBuilder<DefaultActionTypes<Name>> {
// 	pub fn new<C: ActionBuilderComponent>(
// 	) -> ActionBuilder<DefaultActionTypes<C>> {
// 		default()
// 	}
// }


// impl<T: ActionBuilderTypes> ActionBuilder<T> {
// 	// pub fn add_observers(

// 	pub fn on_add(
// 		mut self,
// 		on_add: fn(DeferredWorld, Entity, ComponentId),
// 	) -> Self {
// 		self.on_add = Some(on_add);
// 		self
// 	}
// 	pub fn on_remove(
// 		mut self,
// 		on_remove: fn(DeferredWorld, Entity, ComponentId),
// 	) -> Self {
// 		self.on_remove = Some(on_remove);
// 		self
// 	}

// 	pub fn with_category(mut self, category: ActionCategory) -> Self {
// 		self.category = category;
// 		self
// 	}
// }

// impl<T: ActionBuilderTypes> Plugin for ActionBuilder<T> {
// 	fn build(&self, app: &mut App) {
// 		#[cfg(feature = "reflect")]
// 		app.register_type::<T::Component>();

// 		let world = app.world_mut();
// 		let hooks = world.register_component_hooks::<T::Component>();
// 		if let Some(on_add) = self.on_add {
// 			hooks.on_add(on_add);
// 		}
// 		if let Some(on_remove) = self.on_remove {
// 			hooks.on_remove(on_remove);
// 		}
// 	}
// }
// #[cfg(feature = "reflect")]
// pub trait ActionBuilderComponent: Component + GetTypeRegistration {}
// #[cfg(feature = "reflect")]
// impl<T: Component + GetTypeRegistration> ActionBuilderComponent for T {}
// #[cfg(not(feature = "reflect"))]
// pub trait ActionBuilderComponent: Component {}
// #[cfg(not(feature = "reflect"))]
// impl<T: Component> ActionBuilderComponent for T {}

// pub trait ActionBuilderTypes: 'static + Send + Sync {
// 	type Component: ActionBuilderComponent;
// }
// #[cfg(feature = "reflect")]
// pub struct DefaultActionTypes<T: ActionBuilderComponent>(PhantomData<T>);
// impl<T: ActionBuilderComponent> ActionBuilderTypes for DefaultActionTypes<T> {
// 	type Component = T;
// }
