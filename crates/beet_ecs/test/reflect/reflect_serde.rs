#![allow(unused)]
use beet_ecs::prelude::*;
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_reflect::serde::ReflectSerializer;
use bevy_reflect::serde::TypedReflectDeserializer;
use bevy_reflect::serde::TypedReflectSerializer;
use bevy_reflect::serde::UntypedReflectDeserializer;
use bevy_reflect::FromReflect;
use bevy_reflect::GetTypeRegistration;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
use bevy_scene::serde::SceneDeserializer;
use bevy_scene::serde::SceneSerializer;
use bevy_scene::DynamicScene;
// use bevy_reflect::TypeRegistryArc;
// use bevy_scene::DynamicScene;
use serde::de::DeserializeSeed;
use std::any::TypeId;
// use std::sync::Arc;
// use std::sync::RwLock;
use sweet::*;

#[sweet_test]
fn serde_prefab() -> Result<()> {
	let prefab1 = ConstantScore(Score::Weight(0.5)).into_prefab()?;
	let str1 = ron::ser::to_string_pretty(
		&prefab1,
		ron::ser::PrettyConfig::default(),
	)?;
	let prefab2: TypedBehaviorPrefab<EcsNode> = ron::from_str(&str1)?;
	let str2 = ron::ser::to_string_pretty(
		&prefab2,
		ron::ser::PrettyConfig::default(),
	)?;

	expect(str1).to_be(str2)?;
	Ok(())
}


#[sweet_test]
fn serde_bytes() -> Result<()> {
	let prefab1 = EmptyAction.into_prefab()?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: TypedBehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let bytes2 = bincode::serialize(&prefab2)?;
	expect(bytes1).to_be(bytes2)?;
	Ok(())
}

#[sweet_test]
fn serde_types() -> Result<()> {
	let prefab1 = (Score::default(), ConstantScore::default())
		.child(Score::Weight(0.5))
		.into_prefab()?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: TypedBehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let mut world = World::new();
	let target = world.spawn_empty().id();
	let root = *prefab2.spawn(&mut world, Some(target))?.root().unwrap();
	let child = world.entity(root).get::<Edges>().unwrap()[0];
	expect(&world)
		.component(child)?
		.to_be(&Score::Weight(0.5))?;

	/// these should be in sync with [`append_beet_type_registry`]
	expect(&world).to_have_component::<Name>(root)?;
	expect(&world).to_have_component::<Edges>(root)?;
	expect(&world).to_have_component::<Running>(root)?;
	expect(&world).to_have_component::<RunTimer>(root)?;
	expect(&world).to_have_component::<BehaviorGraphRoot>(root)?;


	Ok(())
}


// #[sweet_test(only)]
// fn serde_scene() -> Result<()> {
// 	let mut world = World::new();
// 	let mut registry = TypeRegistry::new();
// 	registry.register::<ConstantScore>();
// 	let registry = AppTypeRegistry(TypeRegistryArc {
// 		internal: Arc::new(RwLock::new(registry)),
// 	});
// 	world.insert_resource(registry);
// 	let scene = DynamicScene::from_world(&world);
// 	let scene_serializer = SceneSerializer::new(&self.scene, &registry);
// 	scene_serializer.serialize(serializer);

// 	let prefab1 = BehaviorPrefab::<EcsNode>::from_graph(ConstantScore(
// 		Score::Weight(0.5),
// 	))?;
// 	let str1 = ron::ser::to_string_pretty(
// 		&prefab1,
// 		ron::ser::PrettyConfig::default(),
// 	)?;
// 	let prefab2: BehaviorPrefab<EcsNode> = ron::from_str(&str1)?;
// 	let str2 = ron::ser::to_string_pretty(
// 		&prefab2,
// 		ron::ser::PrettyConfig::default(),
// 	)?;
// 	expect(str1).to_be(str2)?;
// 	Ok(())
// }
