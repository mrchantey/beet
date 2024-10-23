use bevy::prelude::*;



#[derive(Component)]
pub struct Foo;

#[derive(Event)]
pub struct Event1;

#[derive(Event)]
pub struct Event2;




fn main() {
	let mut world = World::new();
	world.add_observer(|_trigger: Trigger<Event2>, query: Query<&Foo>| {
		println!("Event2 triggered, num components: {}", query.iter().len());
	});
	world.add_observer(|_trigger: Trigger<Event1>, mut commands: Commands| {
		println!("Event1 triggered");
		// must spawn Foo before trigger
		commands.spawn(Foo);
		commands.trigger(Event2);
	});
	world.flush();
	world.trigger(Event1);
	world.flush();
	println!("Hello, world!");
}
