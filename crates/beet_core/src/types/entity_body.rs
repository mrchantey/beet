//! [`EntityBody`]: one entity's components as data.
use crate::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// One entity's components as data, each keyed by its type path: the shape
/// every entity of a scene takes (`template_serde`'s `{ resources, entities }`
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
/// 	.with(&Provenance::new([ProvenanceSource::file("logo.png", b"logo")]))
/// 	.unwrap();
/// body.get::<Provenance>().unwrap().sources.len().xpect_eq(1);
/// ```
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Deref,
	DerefMut,
	Reflect,
	Serialize,
	serde::Deserialize,
)]
#[reflect(opaque, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityBody(Map);

impl EntityBody {
	/// This body with `component` added, replacing any of its type.
	pub fn with<T: TypePath + Serialize>(
		mut self,
		component: &T,
	) -> Result<Self> {
		self.insert(component)?;
		self.xok()
	}

	/// Add `component`, replacing any of its type.
	pub fn insert<T: TypePath + Serialize>(&mut self, component: &T) -> Result {
		self.0.insert(T::type_path(), Value::from_serde(component)?);
		Ok(())
	}

	/// The component of type `T`, an error naming it when absent, so a
	/// required component reads with `?`.
	pub fn get<T: TypePath + DeserializeOwned>(&self) -> Result<T> {
		match self.0.contains(T::type_path()) {
			true => self.0.get(T::type_path())?.clone().into_serde(),
			false => bevybail!("the body carries no `{}`", T::type_path()),
		}
	}

	/// Whether the body carries a component of type `T`.
	pub fn contains<T: TypePath>(&self) -> bool {
		self.0.contains(T::type_path())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn keys_components_by_type_path() {
		let body = EntityBody::default().with(&Provenance::default()).unwrap();
		body.contains::<Provenance>().xpect_true();
		body.keys()
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("beet_core::atproto::provenance::Provenance");
		EntityBody::default()
			.get::<Provenance>()
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"carries no `beet_core::atproto::provenance::Provenance`",
			);
	}
}
