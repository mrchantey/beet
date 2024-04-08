use crate::prelude::ReflectUtils;
use std::any::type_name;
use std::any::TypeId;
use strum_macros::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, PartialOrd, Ord)]
pub enum BeetType {
	Action,
	Bundle,
	Component,
}




#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BeetTypeInfo {
	pub type_id: TypeId,
	pub name: &'static str,
	pub ty: BeetType,
}
impl BeetTypeInfo {
	pub fn new<T: 'static>(ty: BeetType) -> Self {
		Self {
			type_id: TypeId::of::<T>(),
			name: type_name::<T>(),
			ty,
		}
	}

	// https://github.com/bevyengine/bevy/blob/89a41bc62843be5f92b4b978f6d801af4de14a2d/crates/bevy_reflect/src/type_registry.rs#L156
	pub fn get_short_name(&self) -> String {
		ReflectUtils::get_short_name(self.name)
	}
}

impl Ord for BeetTypeInfo {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.ty.cmp(&other.ty) }
}

impl PartialOrd for BeetTypeInfo {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
