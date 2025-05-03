use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;


/// A wrapper around the [`IkArm4Dof`] that will use the [`GlobalTransform`] pose of
/// the entities to solve the IK.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub struct IkArm4DofTransforms {
	/// Settings for arm parameters (lengths, angles, etc.)
	pub ik: IkArm4Dof,
	/// The target entity to reach.
	pub target: Entity,
	/// The base entity (shoulder) of the arm.
	pub base: Entity,
	/// The first entity (elbow) of the arm.
	pub segment1: Entity,
	/// The second entity (wrist) of the arm.
	pub segment2: Entity,
	/// The third entity (fingertip) of the arm.
	pub segment3: Entity,
}

impl IkArm4DofTransforms {
	/// Create a new [`IkArm4DofTransforms`] with the given entities.
	/// Their locations will resolve each step with the [Self::solve] method.
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

	/// Solve the IK for the target entity using the [`GlobalTransform`] of the entities.
	/// If any of the entities are missing, this will return None.
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
		self.target = entity_mapper.get_mapped(self.target);
		self.base = entity_mapper.get_mapped(self.base);
		self.segment1 = entity_mapper.get_mapped(self.segment1);
		self.segment2 = entity_mapper.get_mapped(self.segment2);
		self.segment3 = entity_mapper.get_mapped(self.segment3);
	}
}

/// A system for updating the [`IkArm4DofTransforms`] based on the target position.
pub(crate) fn update_ik_arm_transforms(
	query: Populated<&IkArm4DofTransforms>,
	global_transforms: Query<&GlobalTransform>,
	mut transforms: Query<&mut Transform>,
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
