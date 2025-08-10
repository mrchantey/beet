use bevy::prelude::*;


#[tokio::main]
async fn main() {
	let mut world = World::new();

	world.insert_resource(Foo(42));
	run_nested(&mut world).await;
}

async fn run_nested(world: &mut World) {
	let value = my_system3(In(true), world).await.unwrap();
	assert_eq!(value, 42);
}

#[derive(Resource, Clone)]
struct Foo(u32);

async fn my_system3(_foo: In<bool>, world: &mut World) -> Result<u32> {
	let foo2 = world.resource::<Foo>();

	tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

	let val = foo2.0;
	Ok(val)
}
