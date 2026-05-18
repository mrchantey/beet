use beet_core::prelude::*;
use bevy::reflect::Typed;


pub enum ClassName {
	TypePath(SmolStr),
	TypeValue {
		type_path: SmolStr,
		type_value: SmolStr,
	},
}

impl ClassName {
	pub fn from_type<T: Typed>() -> Self {
		Self::TypePath(T::type_path().into())
	}
	pub fn from_value<T: Typed + ToString>(value: T) -> Self {
		Self::TypeValue {
			type_path: T::type_path().into(),
			type_value: value.to_string().into(),
		}
	}
}


#[derive(Default, Component, Deref, DerefMut)]
pub struct Classes(HashSet<ClassName>);
