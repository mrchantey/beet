use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Get)]
pub struct Instance {
	/// The value of this instance, matching its
	/// associated schema.
	value: Value,
	/// Schema for the value of this instance.
	/// This may be a reference to an external schema,
	/// which must be available for validation.
	schema: InstanceSchema,
}

pub trait InstancePath {
	fn path(&self) -> &FieldPath;
	/// The schema for this type.
	fn schema(&self) -> &InstanceSchema;
}

#[derive(Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstanceSchema {
	inner: InstanceSchemaInner,
}

impl InstanceSchema {
	pub fn new(schema: Schema) -> Self {
		Self {
			inner: InstanceSchemaInner::Schema(schema),
		}
	}
	pub fn of<T: bevy::reflect::TypePath>() -> Self {
		Self {
			inner: InstanceSchemaInner::Path(SmolStr::new_static(
				T::type_path(),
			)),
		}
	}
}


#[derive(Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum InstanceSchemaInner {
	/// The instance represents a dynamic type built at runtime
	Schema(Schema),
	/// The stable [`TypePath::type_path`] to the type
	/// of this instance.
	/// This is not the [`std::any::type_name`], which
	/// is unstable.
	Path(SmolStr),
}

impl std::fmt::Display for InstanceSchemaInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Path(s) => write!(f, "Path({})", s),
			Self::Schema(_) => write!(f, "Schema(..)"),
		}
	}
}


/// An instance map is like a [`Value`] where branch nodes
/// are nested maps and leaf nodes are typed values.
/// It is perhaps more akin to a filesystem where files are
/// typed, than a freeform json value.
#[derive(Deref)]
pub struct InstanceMap {
	instances: HashMap<FieldPath, Instance>,
}
impl InstanceMap {
	pub fn new() -> Self {
		Self {
			instances: HashMap::new(),
		}
	}
	/// ## Errors
	///
	/// Errors if an existing path exists anywhere up this paths chain,
	/// which would result in overlapping schemas
	pub fn insert(
		&mut self,
		path: FieldPath,
		instance: Instance,
	) -> Result<&mut Self> {
		// check for overlapping paths
		for i in 1..=path.len() {
			let sub_path = FieldPath::from(&path[..i]);
			if self.instances.contains_key(&sub_path) {
				bevybail!(
					"Path {} overlaps with existing path {}",
					path,
					sub_path
				);
			}
		}

		self.instances.insert(path, instance);
		Ok(self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_name() {
		// Name::type_info().type_path().xprintln();
		let _inst = InstanceSchema::of::<Name>();
	}
}
