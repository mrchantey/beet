//! [`EntityBody`]: one entity's components as data.
use crate::prelude::*;
#[cfg(feature = "serde")]
use serde::Serialize;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;

/// One entity's components as data, each keyed by its type path: the shape
/// every entity of a scene takes ([`SceneBody`]'s `{ resources, entities }`
/// holds one per entity), and the whole of a document that is one entity, ie a
/// derived file carrying [`Provenance`].
///
/// A component crosses through its serde form, which for a type reflecting
/// `Serialize` and `Deserialize` is also the form a scene writes. Keys
/// serialize sorted, so a body written twice is byte identical however it was
/// built.
///
/// ```
/// # use beet_core::prelude::*;
/// let body = EntityBody::default()
/// 	.with_typed(&Provenance::new([ProvenanceSource::file(fwd_path!("logo.png"), b"logo")]))
/// 	.unwrap();
/// body.get_typed::<Provenance>().unwrap().sources.len().xpect_eq(1);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(opaque, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct EntityBody(Map);

impl From<Map> for EntityBody {
	fn from(components: Map) -> Self { Self(components) }
}

impl From<EntityBody> for Map {
	fn from(body: EntityBody) -> Self { body.0 }
}

/// The typed accessors: each component crosses through its serde form under
/// its type path. The untyped ones are the [`Map`]'s own (`insert`, `get`,
/// `contains`, `remove`), keyed by the name as a source wrote it.
#[cfg(feature = "serde")]
impl EntityBody {
	/// This body with `component` added, replacing any of its type.
	pub fn with_typed<T: TypePath + Serialize>(
		mut self,
		component: &T,
	) -> Result<Self> {
		self.insert_typed(component)?;
		self.xok()
	}

	/// Add `component`, replacing any of its type.
	pub fn insert_typed<T: TypePath + Serialize>(
		&mut self,
		component: &T,
	) -> Result {
		self.0.insert(T::type_path(), Value::from_serde(component)?);
		Ok(())
	}

	/// The component of type `T`, an error naming it when absent, so a
	/// required component reads with `?`.
	pub fn get_typed<T: TypePath + DeserializeOwned>(&self) -> Result<T> {
		match self.0.contains(T::type_path()) {
			true => self.0.get(T::type_path())?.clone().into_serde(),
			false => bevybail!("the body carries no `{}`", T::type_path()),
		}
	}
}

impl EntityBody {
	/// Whether the body carries a component of type `T`.
	pub fn contains_typed<T: TypePath>(&self) -> bool {
		self.0.contains(T::type_path())
	}
}

/// What a document declares at its root, read without spawning it.
impl EntityBody {
	/// What markup writes of an entity rather than of the document: its node,
	/// its place and its bindings.
	const CONTENT: &'static [&'static str] = &[
		"Attribute",
		"AttributeOf",
		"CData",
		"Cfg",
		"ChildOf",
		"Comment",
		"Doctype",
		"DocumentScope",
		"Element",
		"FieldRef",
		"Instance",
		"OnEvent",
		"Prefab",
		"ProcessingInstruction",
		"ReactiveChildren",
		"ReflectFieldRef",
		"ResourceFieldRef",
		"RunScript",
		"SlotChild",
		"SlotTarget",
		"Value",
	];

	/// The components this body declares rather than is, a document root's
	/// metadata: everything but its content. A document's metadata is so
	/// *whatever components its root declares*, never one blessed type.
	pub fn declarations(&self) -> Self {
		self.iter()
			.filter(|(name, _)| {
				let short = TypeName::new(name.as_str()).short();
				!Self::CONTENT.contains(&short.as_str())
			})
			.map(|(name, value)| (name.clone(), value.clone()))
			.collect::<Map>()
			.into()
	}

	/// Declare `field` of the component short named `component` when this
	/// body holds that component without it: a default something other than
	/// the document implies, ie a content file's numbered filename. Never adds
	/// the component, since a document naming none declared none.
	pub fn declare_default(
		&mut self,
		component: &str,
		field: &str,
		default: Value,
	) {
		let name = self
			.keys()
			.find(|name| TypeName::new(name.as_str()).short() == component)
			.cloned();
		if let Some(Value::Map(fields)) =
			name.and_then(|name| self.0.get_mut(&name))
			&& !fields.contains(field)
		{
			fields.insert(field, default);
		}
	}
}

#[cfg(all(test, feature = "serde"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn keys_components_by_type_path() {
		let body = EntityBody::default()
			.with_typed(&Provenance::default())
			.unwrap();
		body.contains_typed::<Provenance>().xpect_true();
		body.keys()
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("beet_core::atproto::provenance::Provenance");
		EntityBody::default()
			.get_typed::<Provenance>()
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"carries no `beet_core::atproto::provenance::Provenance`",
			);
	}
}
