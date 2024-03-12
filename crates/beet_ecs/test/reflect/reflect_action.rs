use beet_ecs::prelude::*;
use std::any::TypeId;
use sweet::*;

#[derive(Debug, Clone, Reflect, FieldUi, Serialize, Deserialize, PartialEq)]
pub struct SomeValue(pub i32);


#[action(system=my_system)]
struct MyAction {
	pub val: SomeValue,
}

fn my_system() {}


#[sweet_test]
fn recursive_registry() -> Result<()> {
	let mut registry = TypeRegistry::new();
	MyAction::register(&mut registry);
	registry.register::<MyAction>();
	registry.get(TypeId::of::<MyAction>()).unwrap();
	registry
		.get(TypeId::of::<SomeValue>())
		.ok_or_else(|| anyhow::anyhow!("not found"))?;

	Ok(())
}