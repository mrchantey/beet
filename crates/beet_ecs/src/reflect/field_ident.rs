use beet::inspector_options::InspectorTarget;
use bevy::prelude::*;
use bevy::reflect::Access;
use bevy::reflect::ParsedPath;
use std::any::TypeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldIdent {
	pub entity: Entity,
	pub component: TypeId,
	pub path: Vec<Access<'static>>,
	path_parsed: ParsedPath,
}

impl FieldIdent {
	pub fn new(
		entity: Entity,
		component: TypeId,
		path: Vec<Access<'static>>,
	) -> Self {
		Self {
			entity,
			component,
			path_parsed: ParsedPath::from(path.clone()),
			path,
		}
	}
	pub fn path(&self) -> &ParsedPath { &self.path_parsed }
	pub fn child(&self, access: Access<'static>) -> Self {
		let mut path = self.path.clone();
		path.push(access);
		Self::new(self.entity, self.component, path)
	}
	pub fn parent(&self) -> Option<Self> {
		if self.path.len() < 1 {
			return None;
		};
		let mut path = self.path.clone();
		path.pop();
		Some(Self::new(self.entity, self.component, path))
	}

	pub fn inspector_target(
		&self,
		variant_index: Option<usize>,
	) -> Option<InspectorTarget> {
		let Some(access) = self.path.last() else {
			return None;
		};
		// InspectorTarget does not discrimiate between tuple and struct index
		let field_index = match access {
			Access::FieldIndex(index) => *index,
			Access::TupleIndex(index) => *index,
			_ => return None, // named field or list index not supported
		};

		let inspector_target = if let Some(variant_index) = variant_index {
			InspectorTarget::VariantField {
				variant_index,
				field_index,
			}
		} else {
			InspectorTarget::Field(field_index)
		};
		Some(inspector_target)
	}
}
