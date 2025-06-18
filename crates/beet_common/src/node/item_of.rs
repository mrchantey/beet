#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;

/// A component that may or may not have an item of type [`ItemOf<C, T>`].
#[derive(Debug, QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct MaybeWithItem<C: Component, T: 'static + Send + Sync> {
	component: &'static C,
	item: Option<&'static ItemOf<C, T>>,
}


/// A bit like 'Component Relations', this item is related to its target [`C`] component.
/// An example use case is where an entity represents a html element,
/// and it may have a `Span` for only the `NodeTag`, in which case the span
/// is stored as an `ItemOf<NodeTag, Span>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ItemOf<C, T> {
	pub value: T,
	#[reflect(ignore)]
	pub phantom: std::marker::PhantomData<C>,
}


impl<C, T> std::ops::Deref for ItemOf<C, T> {
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.value }
}

impl<C, T> ItemOf<C, T> {
	pub fn new(value: T) -> Self {
		Self {
			value,
			phantom: std::marker::PhantomData,
		}
	}
	pub fn take(self) -> T { self.value }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		#[derive(Debug, Component)]
		struct MyComponent;
		// only component
		world.spawn(MyComponent);
		// only item
		world.spawn(ItemOf::<MyComponent, _>::new(true));
		// both
		world.spawn((MyComponent, ItemOf::<MyComponent, _>::new(true)));


		world
			.query_once::<MaybeWithItemReadOnly<MyComponent, bool>>()
			.xpect()
			.to_have_length(2);
	}
}
