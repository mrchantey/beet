use bevy::ecs::component::ComponentId;
use bevy::prelude::*;
use bevy::reflect::TypeInfo;




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
	pub fn new(value: &dyn Reflect) -> Self {
		value
			.get_represented_type_info()
			.expect("DynComponents are for concrete types");
		Self(value.clone_value())
	}

	pub fn short_name(&self) -> String {
		self.0
			.get_represented_type_info()
			.map(|i| i.type_path_table().short_path())
			.unwrap_or("unknown")
			.to_string()
	}
	pub fn name(&self) -> String {
		heck::AsTitleCase(self.short_name()).to_string()
	}

	pub fn inner(&self) -> &dyn Reflect { self.0.as_ref() }
	pub fn take(self) -> Box<dyn Reflect> { self.0 }

	pub fn get<T: FromReflect>(&self) -> Option<T> {
		T::from_reflect(self.0.as_ref())
	}
	pub fn represents<T: Reflect + TypePath>(&self) -> bool {
		self.0.represents::<T>()
	}
	pub fn represented_type_info(&self) -> &'static TypeInfo {
		self.0
			.get_represented_type_info()
			.expect("DynComponents are for concrete types")
	}

	pub fn component_id(&self, world: &World) -> Option<ComponentId> {
		world
			.components()
			.get_id(self.represented_type_info().type_id())
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
