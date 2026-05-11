use beet_core::prelude::*;
use bevy::reflect::Typed;


#[derive(Deref)]
pub struct ClassName(SmolStr);

impl ClassName {
	pub fn from_type<T: Typed>() -> Self { Self(T::type_path().into()) }
	pub fn from_value<T: Typed + ToString>(value: T) -> Self {
		Self(format!("{}::{}", T::type_path(), value.to_string()).into())
		//
	}
}


#[derive(Default, Component, Deref, DerefMut)]
pub struct ClassSet(HashSet<ClassName>);
