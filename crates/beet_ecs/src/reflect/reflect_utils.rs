use bevy::prelude::*;

pub struct ReflectUtils;

impl ReflectUtils {
	pub fn short_path(val: &dyn Reflect) -> String {
		val.get_represented_type_info()
			.map(|i| i.type_path_table().short_path())
			.unwrap_or("unknown")
			.to_string()
	}
	pub fn name(val: &dyn Reflect) -> String {
		heck::AsTitleCase(Self::short_path(val)).to_string()
	}
	pub fn ident(val: &dyn Reflect) -> String {
		val.get_represented_type_info()
			.map(|i| i.type_path_table().ident())
			.flatten()
			.unwrap_or("unknown")
			.to_string()
	}
	pub fn ident_name(val: &dyn Reflect) -> String {
		heck::AsTitleCase(Self::ident(val)).to_string()
	}
}
