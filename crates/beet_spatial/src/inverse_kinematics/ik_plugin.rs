use crate::prelude::*;
use bevy::prelude::*;


pub fn ik_plugin(app: &mut App) {
	app /*-*/
		.add_systems(Update, (update_ik_arm_transforms,ik_spawner))
		// .add_systems(Update, ik_2dof_transforms_test)
		.register_type::<IkSpawner>()
		.register_type::<IkArm4DofTransforms>()
		/*-*/;
}
