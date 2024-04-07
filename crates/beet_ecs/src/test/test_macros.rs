use crate::prelude::*;
use anyhow::Result;
use std::any::Any;
use sweet::*;

#[derive_action]
#[action(graph_role = GraphRole::Node)]
struct MyStruct;


fn my_struct() {}

#[test]
fn works() -> Result<()> {
	// MyStruct.syste
	let mut registry = TypeRegistry::default();
	MyStruct::register_types(&mut registry);
	expect(registry.get(MyStruct.type_id())).to_be_some()?;

	let mut world = World::new();
	MyStruct::register_bundles(&mut world);
	expect(world.components().get_id(MyStruct.type_id())).to_be_some()?;

	let mut app = App::new();
	MyStruct::add_systems(&mut app, Update);
	expect(app.get_schedule(Update).unwrap().systems_len()).to_be(1)?;

	Ok(())
}
