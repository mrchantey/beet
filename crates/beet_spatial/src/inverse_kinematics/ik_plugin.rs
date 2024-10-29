use crate::prelude::*;
use bevy::prelude::*;


pub fn ik_plugin(app: &mut App) {
	app /*-*/
    .add_plugins(ik_spawner_plugin)
		.add_systems(Update, update_ik_arm_transforms)
		// .add_systems(Update, ik_2dof_transforms_test)
		.register_type::<IkArm4DofTransforms>()
		.register_type::<IkSpawner>()
		/*-*/;
}
