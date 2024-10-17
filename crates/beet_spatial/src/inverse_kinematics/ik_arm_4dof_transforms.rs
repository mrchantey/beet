use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub struct IkArm4DofTransforms {
	pub ik: IkArm4Dof,
	pub target: Entity,
	pub base: Entity,
	pub segment1: Entity,
	pub segment2: Entity,
	pub segment3: Entity,
}

impl IkArm4DofTransforms {
	pub fn new(
		ik: IkArm4Dof,
		target: Entity,
		base: Entity,
		segment1: Entity,
		segment2: Entity,
		segment3: Entity,
	) -> Self {
		Self {
			ik,
			target,
			base,
			segment1,
			segment2,
			segment3,
		}
	}


	pub fn solve(
		&self,
		transforms: &Query<&GlobalTransform>,
	) -> Option<(f32, f32, f32, f32)> {
		let Ok(target) = transforms.get(self.target) else {
			return None;
		};
		let Ok(start) = transforms.get(self.segment1) else {
			return None;
		};

		let delta_pos_including_segment3 =
			target.translation() - start.translation();

		let target_pos_with_segment_3 = Vec3::new(
			delta_pos_including_segment3.x,
			0.0,
			delta_pos_including_segment3.z,
		)
		.normalize()
			* self.ik.segment3.len;

		let delta_pos =
			delta_pos_including_segment3 - target_pos_with_segment_3;


		// TODO delta pos without segment3
		let angles = self.ik.solve4d(delta_pos);

		if angles.0.is_nan()
			|| angles.1.is_nan()
			|| angles.2.is_nan()
			|| angles.3.is_nan()
		{
			None
		} else {
			Some(angles)
		}
	}
}


impl MapEntities for IkArm4DofTransforms {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.target = entity_mapper.map_entity(self.target);
		self.base = entity_mapper.map_entity(self.base);
		self.segment1 = entity_mapper.map_entity(self.segment1);
		self.segment2 = entity_mapper.map_entity(self.segment2);
		self.segment3 = entity_mapper.map_entity(self.segment3);
	}
}


pub fn update_ik_arm_transforms(
	global_transforms: Query<&GlobalTransform>,
	mut transforms: Query<&mut Transform>,
	query: Populated<&IkArm4DofTransforms>,
) {
	for ik_transforms in query.iter() {
		let Some(angles) = ik_transforms.solve(&global_transforms) else {
			log::warn!("IK solver returned NaN, skipping update");
			continue;
		};

		if let Ok(mut seg) = transforms.get_mut(ik_transforms.base) {
			seg.rotation = Quat::from_rotation_y(-angles.0);
		}
		if let Ok(mut seg) = transforms.get_mut(ik_transforms.segment1) {
			seg.rotation = Quat::from_rotation_x(angles.1);
		}
		if let Ok(mut seg) = transforms.get_mut(ik_transforms.segment2) {
			seg.rotation = Quat::from_rotation_x(angles.2);
		}
		if let Ok(mut seg) = transforms.get_mut(ik_transforms.segment3) {
			seg.rotation = Quat::from_rotation_x(angles.3);
		}
	}
}
pub fn ik_arm_transforms_test(
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Populated<&IkArm4DofTransforms>,
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
