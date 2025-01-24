use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::Access;

impl FieldIdent {
	pub fn children(
		&self,
		world: &World,
	) -> Result<Vec<(Option<String>, FieldIdent)>> {
		self.map(world, move |field| {
			let children = match field.reflect_ref() {
				bevy::reflect::ReflectRef::List(val) => {
					list_items(&self, val.iter())
				}
				bevy::reflect::ReflectRef::Array(val) => {
					list_items(&self, val.iter())
				}
				bevy::reflect::ReflectRef::Set(val) => {
					list_items(&self, val.iter())
				}
				bevy::reflect::ReflectRef::Tuple(val) => {
					tuple_items(&self, val.iter_fields())
				}
				bevy::reflect::ReflectRef::TupleStruct(val) => {
					tuple_items(&self, val.iter_fields())
				}
				bevy::reflect::ReflectRef::Struct(val) => {
					struct_items(&self, val)
				}
				bevy::reflect::ReflectRef::Map(val) => {
					map_items(&self, val.iter())
				}
				bevy::reflect::ReflectRef::Enum(_val) => {
					vec![]
				}
				bevy::reflect::ReflectRef::Opaque(_val) => {
					vec![]
				}
			};
			Ok(children)
		})
		.flatten()
	}

	pub fn tree(
		&self,
		world: &World,
		name: Option<String>,
	) -> Result<Tree<(Option<String>, FieldIdent)>> {
		let children = self.children(world)?;

		let children = children
			.into_iter()
			.map(|(name, field)| field.tree(world, name))
			.collect::<Result<Vec<_>>>()?;

		Ok(Tree::new_with_children((name, self.clone()), children))
	}
}


fn struct_items(
	ident: &FieldIdent,
	parent: &dyn Struct,
) -> Vec<(Option<String>, FieldIdent)> {
	parent
		.iter_fields()
		.enumerate()
		.map(|(i, _)| {
			let name = parent.name_at(i).map(|s| s.to_string());
			(name, ident.child(Access::FieldIndex(i)))
		})
		.collect()
}
fn tuple_items<'a>(
	ident: &FieldIdent,
	val: impl Iterator<Item = &'a dyn PartialReflect>,
) -> Vec<(Option<String>, FieldIdent)> {
	val.into_iter()
		.enumerate()
		.map(|(i, _)| (None, ident.child(Access::TupleIndex(i))))
		.collect()
}
fn list_items<'a>(
	ident: &FieldIdent,
	val: impl Iterator<Item = &'a dyn PartialReflect>,
) -> Vec<(Option<String>, FieldIdent)> {
	val.enumerate()
		.map(|(i, _)| (None, ident.child(Access::ListIndex(i))))
		.collect()
}

fn map_items<'a>(
	ident: &FieldIdent,
	val: impl Iterator<Item = (&'a dyn PartialReflect, &'a dyn PartialReflect)>,
) -> Vec<(Option<String>, FieldIdent)> {
	val.enumerate()
		.map(|(i, (k, _))| {
			let name = format!("{k:?}");
			(Some(name), ident.child(Access::ListIndex(i)))
		})
		.collect()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use std::any::TypeId;
	use sweet::prelude::*;

	#[derive(Debug, Default, PartialEq, Component, Reflect)]
	#[reflect(Default, Component)]
	struct MyVecStruct(pub Vec3);

	#[test]
	fn tuple_struct() {
		// setup
		pretty_env_logger::try_init().ok();
		let mut app = App::new();
		app.register_type::<MyVecStruct>();

		let entity = app.world_mut().spawn(MyVecStruct(Vec3::default())).id();


		let field = ComponentIdent::new(entity, TypeId::of::<MyVecStruct>())
			.into_field();

		let tree = field.tree(app.world(), None).unwrap();

		expect(tree.children.len()).to_be(1);
		expect(tree.children[0].children.len()).to_be(3);
	}
}
