use bevy::prelude::*;




/// A version of [`Reflect`] that is [`Clone`] and [`PartialEq`]
#[derive(Deref, DerefMut)]
pub struct DynComponent(Box<dyn Reflect>);

impl Clone for DynComponent {
	fn clone(&self) -> Self { Self(self.clone_value()) }
}
impl PartialEq for DynComponent {
	fn eq(&self, other: &Self) -> bool {
		self.reflect_partial_eq(other.as_ref()).unwrap_or(false)
	}
}

impl DynComponent {
	pub fn new(value: &dyn Reflect) -> Self { Self(value.clone_value()) }

	pub fn inner(&self) -> &dyn Reflect { self.0.as_ref() }
	pub fn take(self) -> Box<dyn Reflect> { self.0 }

	pub fn get<T: FromReflect>(&self) -> Option<T> {
		T::from_reflect(self.0.as_ref())
	}
	pub fn represents<T: Reflect + TypePath>(&self) -> bool {
		self.0.represents::<T>()
	}

	pub fn set<T: Reflect>(&mut self, value: &T) {
		self.0.apply(value)
		// self.0 = value.into_reflect();
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
		let mut dyn_val = DynComponent::new(&val);
		dyn_val.set(&MyStruct(3));
		expect(dyn_val.get::<MyStruct>())
			.as_some()?
			.to_be(MyStruct(3))?;

		Ok(())
	}
}
