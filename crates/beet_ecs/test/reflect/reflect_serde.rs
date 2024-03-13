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

#[derive(Reflect, PartialEq)]
struct Foo(pub usize);

#[derive(Reflect, PartialEq)]
enum Bar {
	A,
	B { a: usize, b: usize },
}

fn do_a_test<T: Reflect + GetTypeRegistration + FromReflect + PartialEq>(
	val: T,
) -> Result<()> {
	let mut registry = TypeRegistry::new();
	registry.register::<T>();
	let registration = registry.get(TypeId::of::<T>()).unwrap();
	let reflect_serializer = TypedReflectSerializer::new(&val, &registry);

	let input = ron::ser::to_string(&reflect_serializer)?;
	let reflect_deserializer =
		TypedReflectDeserializer::new(&registration, &registry);
	let mut deserializer = ron::de::Deserializer::from_str(&input)?;

	let deserialized_value: Box<dyn Reflect> =
		reflect_deserializer.deserialize(&mut deserializer)?;
	Ok(())
}


#[sweet_test]
fn serde_f32() -> Result<()> {
	do_a_test(Foo(0))?;
	do_a_test(Bar::A)?;
	do_a_test(Bar::B { a: 0, b: 0 })?;
	do_a_test(ConstantScore(Score::Weight(0.5)))?;
	Ok(())
}

#[sweet_test(skip)]
fn test_2() -> Result<()> {
	do_another_test(Bazz::B(0.5))?;
	do_another_test(Fizz::B(0.5))?;
	Ok(())
}

#[derive(Reflect, Component)]
#[reflect(Component)]
enum Fizz {
	// #[default]
	B(f32),
	A,
}
#[derive(Reflect, Component)]
#[reflect(Component)]
enum Bazz {
	A,
	B(f32),
}

fn do_another_test<T: Reflect + GetTypeRegistration + Component>(
	val: T,
) -> Result<()> {
	let mut world = World::new();
	world.spawn(val);
	let registry = AppTypeRegistry::default();
	registry.write().register::<T>();
	world.insert_resource(registry.clone());
	let scene1 = DynamicScene::from_world(&world);

	let serialized1 =
		ron::ser::to_string(&SceneSerializer::new(&scene1, &registry))?;

	let mut deserializer = ron::de::Deserializer::from_str(&serialized1)?;
	let scene2 = SceneDeserializer {
		type_registry: &registry.read(),
	}
	.deserialize(&mut deserializer)?;

	let serialized2 =
		ron::ser::to_string(&SceneSerializer::new(&scene2, &registry))?;

	expect(serialized1).to_be(serialized2)?;
	Ok(())
}


// is this the issue? https://github.com/bevyengine/bevy/issues/12357
#[sweet_test(skip)]
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
	println!("1:\n{}", str1);
	println!("2:\n{}", str2);

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
/// these are to be in sync with [`BehaviorPrefab::append_type_registry`]
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

// (
// 	resources: {},
// 	entities: {
// 			4294967296: (
// 					components: {
// 							"beet_ecs::ecs_nodes::actions::scorers::ConstantScore": (Weight(0.5)),
// 					},
// 			),
// 	},
// )
// (
// 	resources: {},
// 	entities: {
// 			4294967296: (
// 					components: {
// 							"beet_ecs::ecs_nodes::actions::scorers::ConstantScore": (Fail(0.5)),
// 					},
// 			),
// 	},
// )
