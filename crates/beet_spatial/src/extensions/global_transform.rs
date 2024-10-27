use bevy::prelude::*;


#[extend::ext(name=GlobalTransformExt)]
pub impl GlobalTransform {
	/// Transform a point from global space to local space.
	/// This temporarily creates an inverse matrix calling [`Mat3A::inverse`], if tranforming many points
	/// consider caching the inverse matrix.
	fn inverse_transform_point(&self, global_point: Vec3) -> Vec3 {
		self.affine().inverse().transform_point3(global_point)
	}
}
