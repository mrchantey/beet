use beet_ecs::prelude::*;
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_reflect::serde::ReflectSerializer;
use bevy_reflect::serde::UntypedReflectDeserializer;
use bevy_reflect::FromReflect;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
// use bevy_reflect::TypeRegistryArc;
// use bevy_scene::DynamicScene;
use serde::de::DeserializeSeed;
// use std::sync::Arc;
// use std::sync::RwLock;
use sweet::*;

#[sweet_test]
fn serde_f32() -> Result<()> {
	let mut registry = TypeRegistry::new();
	registry.register::<ConstantScore>();
	let val1 = ConstantScore::new(Score::Weight(0.5));
	let reflect_serializer = ReflectSerializer::new(&val1, &registry);
	let json = serde_json::to_string(&reflect_serializer)?;
	let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
	let mut json_de = serde_json::Deserializer::from_str(&json);
	let deserialized_value: Box<dyn Reflect> =
		reflect_deserializer.deserialize(&mut json_de)?;
	let val2 =
		<ConstantScore as FromReflect>::from_reflect(&*deserialized_value)
			.unwrap();
	expect(val1).to_be(val2)?;
	Ok(())
}

// is this the issue? https://github.com/bevyengine/bevy/issues/12357
#[sweet_test(skip)]
fn serde_prefab() -> Result<()> {
	let prefab1 = ConstantScore(Score::Weight(0.5)).into_prefab::<EcsNode>()?;
	let str1 = ron::ser::to_string_pretty(
		&prefab1,
		ron::ser::PrettyConfig::default(),
	)?;
	let prefab2: BehaviorPrefab<EcsNode> = ron::from_str(&str1)?;
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
	let prefab1 = EmptyAction.into_prefab::<EcsNode>()?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let bytes2 = bincode::serialize(&prefab2)?;
	expect(bytes1).to_be(bytes2)?;
	Ok(())
}
#[sweet_test]
/// these are to be in sync with [`BehaviorPrefab::append_type_registry`]
fn serde_types() -> Result<()> {
	let prefab1 = (Score::default(), ConstantScore::default())
		.child(Score::Weight(0.5))
		.into_prefab::<EcsNode>()?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
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
