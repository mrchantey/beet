use super::*;
use bevy::prelude::*;

/// A wrapper of [`PartialReflect`] that is [`Clone`] and [`PartialEq`]
#[derive(Deref, DerefMut)]
pub struct DynReflect(pub Box<dyn PartialReflect>);

impl Clone for DynReflect {
	fn clone(&self) -> Self {
		Self(self.clone_value())
	}
}
impl PartialEq for DynReflect {
	fn eq(&self, other: &Self) -> bool {
		self.reflect_partial_eq(other.as_ref()).unwrap_or(false)
	}
}
impl DynReflect {
	pub fn new_cloned(value: &dyn PartialReflect) -> Self {
		Self(value.clone_value())
	}
	pub fn short_path(&self) -> String {
		ReflectUtils::short_path(self.0.as_ref())
	}
	pub fn name(&self) -> String { ReflectUtils::name(self.0.as_ref()) }
	pub fn try_into_reflect<T: FromReflect>(&self) -> Option<T> {
		T::from_reflect(self.0.as_ref())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[derive(Debug, Clone, PartialEq, Reflect)]
	struct MyStruct(pub i32);

	#[test]
	fn works() -> Result<()> {
		let val = MyStruct(7);
		let mut dyn_val = DynReflect::new_cloned(&val);
		dyn_val.apply(&MyStruct(3));
		expect(dyn_val.try_into_reflect::<MyStruct>())
			.as_some()?
			.to_be(MyStruct(3))?;

		Ok(())
	}
}
