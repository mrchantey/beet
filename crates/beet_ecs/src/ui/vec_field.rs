use crate::prelude::*;
use bevy_math::prelude::*;

// macro_rules! field_ui_macro {
// 	($reflect:expr, $field_name:expr, $ident:expr) => {
// 		FieldReflect::new(
// 			$field_name.to_string(),
// 			{
// 				let get_cb = $reflect.clone_get_cb();
// 				move || get_cb().$ident.clone()
// 			},
// 			{
// 				let get_cb = $reflect.clone_get_cb();
// 				let set_cb = $reflect.clone_set_cb();
// 				move |val| {
// 					let mut parent = get_cb();
// 					parent.$ident = val;
// 					set_cb(parent);
// 				}
// 			},
// 		)
// 	};
// }

impl IntoFieldUi for Vec3 {
	fn into_field_ui(_reflect: FieldReflect<Self>) -> FieldUi {
		todo!()
		// GroupField::new(reflect.display_name.clone(), vec![
		// 	// f32::into_field_ui(
		// 	// field_ui_macro!(reflect, "x", x),
		// // )
		// ])
		// .into()
	}
}
