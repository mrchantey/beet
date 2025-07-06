use bevy::prelude::*;

/// Extension methods for [`GlobalTransform`].
pub struct GlobalTransformExt;

impl GlobalTransformExt {
	/// Transform a point from global space to local space.
	/// This temporarily creates an inverse matrix calling [`Mat3A::inverse`], if tranforming many points
	/// consider caching the inverse matrix.
	pub fn inverse_transform_point(
		transform: &GlobalTransform,
		global_point: Vec3,
	) -> Vec3 {
		transform.affine().inverse().transform_point3(global_point)
	}
}
