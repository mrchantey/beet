use crate::prelude::*;
use bevy::prelude::*;


pub fn ik_plugin(app: &mut App) {
	app /*-*/
		.add_systems(Update, ik_2dof_transforms)
		// .add_systems(Update, ik_2dof_transforms_test)
		.register_type::<Ik2DofTransforms>()
		/*-*/;
}
