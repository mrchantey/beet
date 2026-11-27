//! [`ReflectName`]: a type as reflection names it.
use crate::prelude::*;
use bevy::reflect::TypePath;
use bevy::reflect::TypePathTable;
use bevy::reflect::TypeRegistration;
use bevy::reflect::TypeRegistry;
use core::ops::Deref;
use smol_str::ToSmolStr;

/// A type as reflection names it, what a registry resolves and a scene stores:
/// a [`TypePath`], or the name a document wrote, which may be short as a tag
/// is.
///
/// Distinct from [`TypeName`], which is `core::any::type_name`'s display
/// string for diagnostics. The two differ for a type a function declares
/// (`..::{{closure}}::Foo` against `..::Foo`), so neither converts into the
/// other and a name a registry reads is always this one.
///
/// ```
/// # use beet_core::prelude::*;
/// let name = ReflectName::of::<Name>();
/// name.as_str().xpect_eq("bevy_ecs::name::Name");
/// name.short().xpect_eq("Name");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct ReflectName(SmolStr);

impl ReflectName {
	/// The name `T` is registered under, its [`TypePath`].
	pub fn of<T: ?Sized + TypePath>() -> Self {
		Self(SmolStr::new_static(T::type_path()))
	}

	/// A name as written, full or short.
	pub fn new(name: impl Into<SmolStr>) -> Self { Self(name.into()) }

	/// The name as written.
	pub fn as_str(&self) -> &str { self.0.as_str() }

	/// The short name, every path segment shortened, generics included, the
	/// name markup authors a tag by.
	pub fn short(&self) -> SmolStr { ShortName(&self.0).to_smolstr() }

	/// The registration this names in `registry`, by full path or, as markup
	/// writes a tag, by short path.
	pub fn registration<'a>(
		&self,
		registry: &'a TypeRegistry,
	) -> Option<&'a TypeRegistration> {
		registry
			.get_with_type_path(self.as_str())
			.or_else(|| reflect_ext::registration_by_name(registry, self))
	}

	/// [`registration`](Self::registration) for a type an entity carries, so
	/// an ambiguous short name resolves to its sole component or resource, see
	/// [`reflect_ext::carried_registration_by_name`].
	pub fn carried_registration<'a>(
		&self,
		registry: &'a TypeRegistry,
	) -> Option<&'a TypeRegistration> {
		registry
			.get_with_type_path(self.as_str())
			.or_else(|| reflect_ext::carried_registration_by_name(registry, self))
	}

	/// Whether this names the type at `path`, by its full path or the short
	/// one a document wrote.
	pub fn names(&self, path: &TypePathTable) -> bool {
		self.as_str() == path.path() || self.as_str() == path.short_path()
	}
}

impl core::fmt::Display for ReflectName {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.0)
	}
}

impl Deref for ReflectName {
	type Target = str;
	fn deref(&self) -> &str { self.0.as_str() }
}

impl AsRef<str> for ReflectName {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl From<&str> for ReflectName {
	fn from(name: &str) -> Self { Self::new(name) }
}

impl From<SmolStr> for ReflectName {
	fn from(name: SmolStr) -> Self { Self(name) }
}

impl From<ReflectName> for SmolStr {
	fn from(name: ReflectName) -> Self { name.0 }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A type a function declares is named as reflection registers it, which
	/// is not the path `core::any::type_name` gives it.
	#[crate::test]
	fn names_a_local_type_as_registered() {
		#[derive(Reflect)]
		struct Local;
		let mut registry = bevy::reflect::TypeRegistry::default();
		registry.register::<Local>();
		ReflectName::of::<Local>()
			.registration(&registry)
			.xpect_some();
		(TypeName::of::<Local>().as_str()
			== ReflectName::of::<Local>().as_str())
		.xpect_false();
		ReflectName::new("Local")
			.registration(&registry)
			.xpect_some();
	}
}
