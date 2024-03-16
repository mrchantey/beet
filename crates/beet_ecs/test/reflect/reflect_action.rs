use beet_ecs::prelude::*;
use std::any::TypeId;
use sweet::*;

#[derive(Debug, Clone, Reflect)]
pub struct SomeValue(pub i32);

#[derive_action]
struct MyAction {
	pub val: SomeValue,
}

fn my_action() {}


#[sweet_test]
fn recursive_registry() -> Result<()> {
	let mut registry = TypeRegistry::new();
	MyAction::register_types(&mut registry);
	registry.register::<MyAction>();
	registry.get(TypeId::of::<MyAction>()).unwrap();
	registry
		.get(TypeId::of::<SomeValue>())
		.ok_or_else(|| anyhow::anyhow!("not found"))?;

	Ok(())
}
