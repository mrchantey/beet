// use bevy_derive::Deref;
// use bevy_derive::DerefMut;
// use bevy_ecs::entity::Entity;
// use bevy_ecs::entity::EntityHashMap;
// use bevy_ecs::entity::EntityMapper;
// use bevy_ecs::entity::MapEntities;
// use bevy_ecs::reflect::ReflectMapEntities;
// use bevy_ecs::reflect::ReflectResource;
// use bevy_ecs::system::Resource;
// use bevy_reflect::Reflect;

// /// This should only be used to temporarily store root of a behavior
// #[derive(Resource, Deref, DerefMut, Reflect)]
// #[reflect(Resource, MapEntities)]
// pub struct SerdeRootEntity(pub Entity);
// impl Default for SerdeRootEntity {
// 	fn default() -> Self { Self(Entity::PLACEHOLDER) }
// }

// impl MapEntities for SerdeRootEntity {
// 	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
// 		**self = entity_mapper.map_entity(**self);
// 	}
// }
