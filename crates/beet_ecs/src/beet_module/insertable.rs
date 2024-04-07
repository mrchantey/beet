// use crate::prelude::ReflectActionMeta;
// use bevy::prelude::*;
// use bevy::reflect::TypeInfo;
// use bevy::reflect::TypeRegistry;
// use std::any::TypeId;

// #[derive(Debug, Clone)]
// pub enum InsertableType {
// 	Action,
// 	Component,
// 	Bundle,
// }

// #[derive(Debug, Clone)]
// pub struct Insertable {
// 	pub insertable_type: InsertableType,
// 	pub info: &'static TypeInfo,
// 	pub components: Vec<TypeId>,
// }

// impl Insertable {
// 	pub fn get_all(world: &mut World) -> Vec<Self> {
// 		let registry = world.resource::<AppTypeRegistry>().read();
// 		let mut insertables = world
// 			.components()
// 			.iter()
// 			.filter_map(|info| registry.get_type_info(info.type_id().unwrap()))
// 			.filter_map(|info| registry.get(info.type_id()))
// 			.filter(|registration| {
// 				registration.data::<ReflectDefault>().is_some()
// 			})
// 			.map(|registration| {
// 				let insertable_type =
// 					if registration.data::<ReflectActionMeta>().is_some() {
// 						InsertableType::Action
// 					} else {
// 						InsertableType::Component
// 					};
// 				Self {
// 					insertable_type,
// 					components: vec![registration.type_id()],
// 					info: registration.type_info(),
// 				}
// 			}).collect::<Vec<_>>();

// 		// 	} else if registration.data::<ReflectActionMeta>().is_none() {
// 		// 		false
// 		// 	} else {
// 		// 		true
// 		// 	}
// 		// })
// 		// .map(|registration| )
// 		// .collect()
// 	}
// }
