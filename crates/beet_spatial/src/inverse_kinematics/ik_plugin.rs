use crate::prelude::*;
use beet_core::prelude::*;

/// Add the update methods for IK.
pub fn ik_plugin(app: &mut App) {
	app /*-*/
		.add_systems(Update, update_ik_arm_transforms)
		// .add_systems(Update, ik_2dof_transforms_test)
		.register_type::<IkArm4DofTransforms>()
		/*-*/;

	#[cfg(feature = "bevy_default")]
	app.add_plugins(ik_spawner_plugin);
}
