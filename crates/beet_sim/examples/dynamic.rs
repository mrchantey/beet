use bevy::ecs::component::ComponentCloneBehavior;
use bevy::ecs::component::ComponentDescriptor;
use bevy::ecs::component::StorageType;
use bevy::ecs::world::FilteredEntityMut;
use beet_core::prelude::*;
use std::alloc::Layout;

fn main() {
	let mut world = World::new();

	// 1. Register a dynamic component
	// SAFETY: Using u64 which is Send + Sync
	let descriptor = unsafe {
		ComponentDescriptor::new_with_layout(
			"DynamicComp".to_string(),
			StorageType::Table,
			Layout::array::<u64>(2).unwrap(),
			None,
			true,
			ComponentCloneBehavior::Default,
		)
	};
	let comp_id = world.register_component_with_descriptor(descriptor);
	// 2. Create entity with the dynamic component
	let mut entity = world.spawn_empty();
	let data = vec![42u64, 123u64];

	// SAFETY: Component created with correct layout
	unsafe {
		entity.insert_by_id(
			comp_id,
			bevy::ptr::OwningPtr::new(std::ptr::NonNull::new_unchecked(
				data.as_ptr() as *mut _,
			)),
		);
	}

	// 3. Query for the component
	let mut query_builder = QueryBuilder::<FilteredEntityMut>::new(&mut world);
	query_builder.ref_id(comp_id);
	let mut query = query_builder.build();

	// 4. Iterate and print results
	for entity in query.iter(&world) {
		if let Some(ptr) = entity.get_by_id(comp_id) {
			// SAFETY: We know the layout is [u64; 2]
			let data = unsafe {
				std::slice::from_raw_parts(ptr.as_ptr().cast::<u64>(), 2)
			};
			println!("Entity data: {:?}", data);
		}
	}


	world.insert_resource(AppTypeRegistry::default());
	let scene = DynamicScene::from_world(&world);
	let type_registry = world.get_resource::<AppTypeRegistry>().unwrap();
	let type_registry = type_registry.read();

	let serialized_scene = scene.serialize(&type_registry).unwrap();

	println!("Serialized scene: {:?}", serialized_scene);
}
