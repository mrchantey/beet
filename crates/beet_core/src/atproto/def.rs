//! [`Def`]: a type's name in a record, and the registry's lookup both ways.
use crate::prelude::*;
use bevy::reflect::CreateTypeData;
use bevy::reflect::TypeRegistration;
use bevy::reflect::TypeRegistry;
use core::any::TypeId;

/// A type that may appear in a record, named by its NSID: the lexicon its
/// crate exports under, `#`, and the type name camel cased,
/// `org.beet.core#provenance`. A bare lexicon is a def too, its `main` the
/// type: `org.beet.scene`.
///
/// Derived, never written by hand (`#[derive(Def)]`, see its docs for the
/// naming rules); `#[reflect(Def)]` registers [`ReflectDef`], so a registry
/// answers the type of an NSID and the NSID of a type. A component is a def
/// used as a record's key, which is what makes a plugin's component
/// self-describing to any tool.
///
/// ```
/// # use beet_core::prelude::*;
/// Provenance::NSID.as_str().xpect_eq("org.beet.core#provenance");
/// ```
pub trait Def {
	/// The def's name.
	const NSID: Nsid;
}

/// [`Def`] as type data, registered by `#[reflect(Def)]`: the NSID a record
/// keys this type by.
#[derive(Debug, Clone)]
pub struct ReflectDef {
	nsid: Nsid,
}

impl ReflectDef {
	/// The NSID this type is keyed by.
	pub fn nsid(&self) -> &Nsid { &self.nsid }

	/// The registration of the type `nsid` names, `None` when no registered
	/// def has it.
	pub fn registration<'a>(
		registry: &'a TypeRegistry,
		nsid: &Nsid,
	) -> Option<&'a TypeRegistration> {
		registry.iter().find(|registration| {
			registration
				.data::<Self>()
				.is_some_and(|def| &def.nsid == nsid)
		})
	}

	/// The NSID of the type `type_id`, `None` when it is no registered def.
	pub fn nsid_of(registry: &TypeRegistry, type_id: TypeId) -> Option<Nsid> {
		registry
			.get_type_data::<Self>(type_id)
			.map(|def| def.nsid.clone())
	}
}

impl<T: Def> CreateTypeData<T> for ReflectDef {
	fn create_type_data(_input: ()) -> Self { Self { nsid: T::NSID } }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::reflect::TypeRegistry;
	use core::any::TypeId;

	#[derive(Def, Reflect)]
	#[reflect(Def)]
	struct LoadSetsLike;

	#[derive(Def, Reflect)]
	#[def(lexicon = "com.example.plugin")]
	#[reflect(Def)]
	struct Thing;

	#[derive(Def)]
	#[def(nsid = "com.example.record")]
	struct Record;

	#[crate::test]
	fn names_by_crate_and_type() {
		LoadSetsLike::NSID
			.as_str()
			.xpect_eq("org.beet.core#loadSetsLike");
		Thing::NSID.as_str().xpect_eq("com.example.plugin#thing");
		Record::NSID.as_str().xpect_eq("com.example.record");
		// every derived name is a valid nsid
		for nsid in [LoadSetsLike::NSID, Thing::NSID, Record::NSID] {
			Nsid::parse(nsid.as_str()).unwrap();
		}
	}

	#[crate::test]
	fn looks_up_both_ways() {
		let mut registry = TypeRegistry::default();
		registry.register::<Thing>();
		ReflectDef::nsid_of(&registry, TypeId::of::<Thing>())
			.xpect_eq(Some(Thing::NSID));
		ReflectDef::registration(&registry, &Thing::NSID)
			.unwrap()
			.type_id()
			.xpect_eq(TypeId::of::<Thing>());
		ReflectDef::registration(&registry, &LoadSetsLike::NSID).xpect_none();
	}
}
