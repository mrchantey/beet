use crate::prelude::*;
use bevy::prelude::*;
use extend::ext;
use std::ops::Deref;

#[ext(name=MatcherMutExtWorld)]
/// Matcher extensions for `bevy::World`
pub impl<'a, W> Matcher<W>
where
	W: 'a + Into<&'a mut World>,
{
	fn num_components<T: Component>(self) -> Matcher<usize> {
		let world = self.value.into();
		let mut arr = world.query::<&T>();
		let received = arr.iter(world).count();
		Matcher::new(received)
	}
}

#[ext(name=MatcherExtWorld)]
/// Matcher extensions for `bevy::World`
pub impl<'a, W> Matcher<W>
where
	W: 'a + Deref<Target = World>,
{
	fn to_have_entity(&self, entity: Entity) {
		let value = self.value.deref();
		let received = value.get_entity(entity);
		self.assert_option_with_received_negatable(received.ok());
	}

	fn to_have_component<T: Component>(&self, entity: Entity) {
		let received = self.value.deref().get::<T>(entity);
		self.assert_option_with_received_negatable(received);
	}

	fn component<T: Component>(&self, entity: Entity) -> Matcher<&T> {
		let received = self.value.deref().get::<T>(entity);
		self.assert_some_with_received(received);
		Matcher::new(received.unwrap())
	}

	fn to_contain_resource<T: Resource>(&self) {
		let received = self.value.deref().get_resource::<T>();
		self.assert_option_with_received_negatable(received);
	}

	fn resource<T: Resource>(&self) -> Matcher<&T> {
		let received = self.value.deref().get_resource::<T>();
		self.assert_some_with_received(received);
		Matcher::new(received.unwrap())
	}

	fn to_contain_non_send_resource<T: 'static>(&self) {
		let received = self.value.deref().get_non_send_resource::<T>();
		self.assert_option_with_received_negatable(received);
	}

	fn non_send_resource<T: 'static>(&self) -> Matcher<&T> {
		let received = self.value.deref().get_non_send_resource::<T>();
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
		expect(&world).not().to_contain_resource::<Health>();
		world.insert_resource(Health(5));
		expect(&world).to_contain_resource::<Health>();
	}

	#[test]
	fn app() {
		let mut app = App::new();
		let entity = app.world_mut().spawn_empty().id();

		expect(app.world())
			.not()
			.to_have_component::<Health>(entity);
		app.world_mut().entity_mut(entity).insert(Health(7));
		expect(app.world()).to_have_component::<Health>(entity);
		expect(app.world())
			.component::<Health>(entity)
			.to_be(&Health(7));

		expect(app.world()).not().to_contain_resource::<Health>();
		app.world_mut().insert_resource(Health(5));
		expect(app.world()).to_contain_resource::<Health>();
		expect(app.world()).resource::<Health>().to_be(&Health(5));

		expect(app.world())
			.not()
			.to_contain_non_send_resource::<Health>();
		app.world_mut().insert_non_send_resource(Health(5));
		expect(app.world()).to_contain_non_send_resource::<Health>();
		expect(app.world())
			.non_send_resource::<Health>()
			.to_be(&Health(5));
	}
}
