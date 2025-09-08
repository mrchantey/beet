use crate::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use extend::ext;

sweet_ref_impls!(World);
impl SweetRef<World> for App {
	fn sweet_ref(&self) -> &World { self.world() }
}
impl SweetMut<World> for App {
	fn sweet_mut(&mut self) -> &mut World { self.world_mut() }
}
impl SweetRef<World> for &App {
	fn sweet_ref(&self) -> &World { self.world() }
}
impl SweetRef<World> for &mut App {
	fn sweet_ref(&self) -> &World { self.world() }
}
impl SweetMut<World> for &mut App {
	fn sweet_mut(&mut self) -> &mut World { self.world_mut() }
}


#[ext(name=WorldMutExtMatcher)]
/// Matcher extensions for `bevy::World`
pub impl<W: SweetMut<World>> Matcher<W> {
	fn query<'a, D: QueryData>(&'a mut self) -> Vec<D::Item<'a>> {
		let world = self.value.sweet_mut();
		world.query::<D>().iter_mut(world).collect::<Vec<_>>()
	}
	fn num_components<T: Component>(&mut self) -> Matcher<usize> {
		let world = self.value.sweet_mut();
		let mut arr = world.query::<&T>();
		let received = arr.iter(world).count();
		Matcher::new(received)
	}
}

#[ext(name=MatcherExtWorld)]
/// Matcher extensions for `bevy::World`
pub impl<W: SweetRef<World>> Matcher<W> {
	fn to_have_entity(&self, entity: Entity) {
		let value = self.value.sweet_ref();
		let received = value.get_entity(entity);
		self.assert_option_with_received_negatable(received.ok());
	}

	fn to_have_component<T: Component>(&self, entity: Entity) {
		let received = self.value.sweet_ref().get::<T>(entity);
		self.assert_option_with_received_negatable(received);
	}

	fn component<T: Component>(&self, entity: Entity) -> Matcher<&T> {
		let received = self.value.sweet_ref().get::<T>(entity);
		self.assert_some_with_received(received);
		Matcher::new(received.unwrap())
	}

	fn to_contain_resource<T: Resource>(&self) {
		let received = self.value.sweet_ref().get_resource::<T>();
		self.assert_option_with_received_negatable(received);
	}

	fn resource<T: Resource>(&self) -> Matcher<&T> {
		let received = self.value.sweet_ref().get_resource::<T>();
		self.assert_some_with_received(received);
		Matcher::new(received.unwrap())
	}

	fn to_contain_non_send_resource<T: 'static>(&self) {
		let received = self.value.sweet_ref().get_non_send_resource::<T>();
		self.assert_option_with_received_negatable(received);
	}

	fn non_send_resource<T: 'static>(&self) -> Matcher<&T> {
		let received = self.value.sweet_ref().get_non_send_resource::<T>();
		self.assert_some_with_received(received);
		Matcher::new(received.unwrap())
	}

	//breaks backtracing
	// fn component_to_be<T>(
	// 	&self,
	// 	entity: impl SweetInto<Entity>,
	// 	other: &T,
	// )
	// where
	// 	T: Component + PartialEq + std::fmt::Debug,
	// {
	// 	self.component::<T>(entity)?.to_be(other)
	// }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[derive(Debug, PartialEq, Component, Resource)]
	pub struct Health(pub u32);

	#[test]
	fn world() {
		let mut world = World::new();
		(&world).xpect().not().to_contain_resource::<Health>();
		world.insert_resource(Health(5));
		(&world).xpect().to_contain_resource::<Health>();
	}

	#[test]
	fn app() {
		let mut app = App::new();
		let entity = app.world_mut().spawn_empty().id();

		app.world()
			.xpect()
			.not()
			.to_have_component::<Health>(entity);
		app.world_mut().entity_mut(entity).insert(Health(7));
		app.world().xpect().to_have_component::<Health>(entity);
		app.world()
			.xpect()
			.component::<Health>(entity)
			.to_be(&Health(7));

		app.world().xpect().not().to_contain_resource::<Health>();
		app.world_mut().insert_resource(Health(5));
		app.world().xpect().to_contain_resource::<Health>();
		app.world().xpect().resource::<Health>().to_be(&Health(5));

		app.world()
			.xpect()
			.not()
			.to_contain_non_send_resource::<Health>();
		app.world_mut().insert_non_send_resource(Health(5));
		app.world().xpect().to_contain_non_send_resource::<Health>();
		app.world()
			.xpect()
			.non_send_resource::<Health>()
			.to_be(&Health(5));
	}
}
