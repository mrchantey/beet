//! [`LexiconDef`]: a type's name in a record, and the registry's lookup both ways.
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
/// Derived, never written by hand (`#[derive(LexiconDef)]`, see its docs for the
/// naming rules); `#[reflect(LexiconDef)]` registers [`ReflectLexiconDef`], so a registry
/// answers the type of an NSID and the NSID of a type. A component is a def
/// used as a record's key, which is what makes a plugin's component
/// self-describing to any tool.
///
/// ```
/// # use beet_core::prelude::*;
/// Provenance::NSID.as_str().xpect_eq("org.beet.core#provenance");
/// ```
pub trait LexiconDef {
	/// The def's name.
	const NSID: Nsid;
}

/// [`LexiconDef`] as type data, registered by `#[reflect(LexiconDef)]`: the NSID a record
/// keys this type by.
#[derive(Debug, Clone)]
pub struct ReflectLexiconDef {
	nsid: Nsid,
}

impl ReflectLexiconDef {
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

impl<T: LexiconDef> CreateTypeData<T> for ReflectLexiconDef {
	fn create_type_data(_input: ()) -> Self { Self { nsid: T::NSID } }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::reflect::TypeRegistry;
	use core::any::TypeId;

	#[derive(LexiconDef, Reflect)]
	#[reflect(LexiconDef)]
	struct LoadSetsLike;

	#[derive(LexiconDef, Reflect)]
	#[lexicon_def(lexicon = "com.example.plugin")]
	#[reflect(LexiconDef)]
	struct Thing;

	#[derive(LexiconDef)]
	#[lexicon_def(nsid = "com.example.record")]
	struct Record;

	#[crate::test]
	fn names_by_crate_and_type() {
		LoadSetsLike::NSID
			.as_str()
			.xpect_eq("org.beet.core#loadSetsLike");
		Thing::NSID.as_str().xpect_eq("com.example.plugin#thing");
		Record::NSID.as_str().xpect_eq("com.example.record");
	}

	#[crate::test]
	fn looks_up_both_ways() {
		let mut registry = TypeRegistry::default();
		registry.register::<Thing>();
		ReflectLexiconDef::nsid_of(&registry, TypeId::of::<Thing>())
			.xpect_eq(Some(Thing::NSID));
		ReflectLexiconDef::registration(&registry, &Thing::NSID)
			.unwrap()
			.type_id()
			.xpect_eq(TypeId::of::<Thing>());
		ReflectLexiconDef::registration(&registry, &LoadSetsLike::NSID)
			.xpect_none();
	}
}
