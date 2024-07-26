use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::Access;
use bevy::reflect::ParsedPath;
use bevy::reflect::ReflectRef;
use std::any::TypeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldIdent {
	pub component: ComponentIdent,
	pub path: Vec<Access<'static>>,
	path_parsed: ParsedPath,
}

impl FieldIdent {
	pub fn new(component: ComponentIdent, path: Vec<Access<'static>>) -> Self {
		Self {
			component,
			path_parsed: ParsedPath::from(path.clone()),
			path,
		}
	}
	pub fn new_with_entity(
		entity: Entity,
		type_id: TypeId,
		path: Vec<Access<'static>>,
	) -> Self {
		Self::new(ComponentIdent::new(entity, type_id), path)
	}

	pub fn path(&self) -> &ParsedPath { &self.path_parsed }
	pub fn child(&self, access: Access<'static>) -> Self {
		let mut path = self.path.clone();
		path.push(access);
		Self::new(self.component, path)
	}
	pub fn parent(&self) -> Option<Self> {
		if self.path.len() < 1 {
			return None;
		};
		let mut path = self.path.clone();
		path.pop();
		Some(Self::new(self.component, path))
	}

	pub fn map<O>(
		&self,
		world: &World,
		func: impl FnOnce(&dyn Reflect) -> O,
	) -> Result<O> {
		self.component.map(world, |component| {
			let field = component
				.reflect_path(self.path())
				.map_err(|e| anyhow::anyhow!("{e}"))?;
			Ok(func(field))
		})?
	}
	pub fn map_mut<O>(
		&self,
		world: &mut World,
		func: impl FnOnce(&mut dyn Reflect) -> O,
	) -> Result<O> {
		self.component.map_mut(world, |component| {
			let field = component
				.reflect_path_mut(self.path())
				.map_err(|e| anyhow::anyhow!("{e}"))?;
			Ok(func(field))
		})?
	}
	pub fn get(&self, world: &World) -> Result<Box<dyn Reflect>> {
		self.map(world, |c| c.clone_value())
	}

	pub fn set(
		&self,
		world: &mut World,
		new_value: &dyn Reflect,
	) -> Result<()> {
		self.map_mut(world, move |field| {
			field.apply(new_value);
			Ok(())
		})?
	}

	pub fn variant_index(&self, world: &World) -> Result<usize> {
		self.map(world, |field| match field.reflect_ref() {
			ReflectRef::Enum(field) => Ok(field.variant_index()),
			_ => Err(anyhow::anyhow!("field is not an enum")),
		})?
	}

}
