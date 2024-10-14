use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;



#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub struct Ik2DofTransforms {
	pub ik: Ik2Dof,
	pub target: Entity,
	pub segment1: Entity,
	pub segment2: Entity,
}

impl Ik2DofTransforms {
	pub fn new(
		ik: Ik2Dof,
		target: Entity,
		segment1: Entity,
		segment2: Entity,
	) -> Self {
		Self {
			ik,
			target,
			segment1,
			segment2,
		}
	}
}


impl MapEntities for Ik2DofTransforms {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.target = entity_mapper.map_entity(self.target);
		self.segment1 = entity_mapper.map_entity(self.segment1);
		self.segment2 = entity_mapper.map_entity(self.segment2);
	}
}


pub fn ik_2dof_transforms(
	mut transforms: Query<&mut Transform>,
	query: Populated<&Ik2DofTransforms>,
) {
	for ik_transforms in query.iter() {
		let Ok(target) = transforms.get(ik_transforms.target) else {
			continue;
		};
		// TODO this will need to be rotated etc in 3d
		let pos = target.translation.truncate();
		let (angle1, angle2) = ik_transforms.ik.solve(pos);

		if let Ok(mut segment1) = transforms.get_mut(ik_transforms.segment1) {
			segment1.rotation = Quat::from_rotation_x(angle1);
		}
		if let Ok(mut segment2) = transforms.get_mut(ik_transforms.segment2) {
			segment2.rotation = Quat::from_rotation_x(angle2);
		}
	}
}
pub fn ik_2dof_transforms_test(
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Populated<&Ik2DofTransforms>,
) {
	for ik_transforms in query.iter() {
		if let Ok(mut segment1) = transforms.get_mut(ik_transforms.segment1) {
			let angle1 = time.elapsed_seconds().sin();
			// let dir = Vec3::new(angle1.cos(), angle1.sin(), 0.0);
			segment1.rotation = Quat::from_rotation_x(angle1);
			// segment1.rotation = Quat::look_at(dir);
		}
		if let Ok(mut segment2) = transforms.get_mut(ik_transforms.segment2) {
			let angle2 = time.elapsed_seconds().sin();
			// let dir = Vec3::new(angle2.cos(), angle2.sin(), 0.0);
			// segment2.translation = Vec3::new(ik_transforms.ik.segment1.len, 0.0, 0.0);
			// segment2.rotation = Quat::look_at(dir);
			segment2.rotation = Quat::from_rotation_x(angle2);
		}
	}
}
