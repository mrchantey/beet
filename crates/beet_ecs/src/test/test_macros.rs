use crate::prelude::*;
use anyhow::Result;
use std::any::Any;
use sweet::*;


struct Bazz {
	val: f32,
}

#[derive(Action)]
struct Fizz<T> {
	bazz: Bazz,
	val: T,
}

fn fizz<T>() {}


#[derive_action]
#[action(graph_role = GraphRole::Node)]
struct MyStruct;



#[derive(BeetModule)]
#[actions(MyStruct)]
struct MyModule;

fn my_struct() {}

#[test]
fn works() -> Result<()> {
	let mut registry = TypeRegistry::default();
	MyModule::register_types(&mut registry);
	expect(registry.get(MyStruct.type_id())).to_be_some()?;

	let mut world = World::new();
	MyModule::register_bundles(&mut world);
	expect(world.components().get_id(MyStruct.type_id())).to_be_some()?;

	let mut app = App::new();
	MyModule::add_systems(&mut app, Update);
	expect(app.get_schedule(Update).unwrap().systems_len()).to_be(1)?;

	let mut app = App::new();
	MyStruct::add_systems(&mut app, Update);
	expect(app.get_schedule(Update).unwrap().systems_len()).to_be(1)?;

	Ok(())
}
