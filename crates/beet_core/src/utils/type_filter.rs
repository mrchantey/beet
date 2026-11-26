//! [`TypeFilter`]: which types pass, by exact name.
use crate::prelude::*;
use bevy::reflect::TypePath;
use bevy::reflect::TypePathTable;

/// Which types pass, by [`ReflectName`]: every type, the types allowed, or
/// every type but those denied. A lookup per type, matched against its full
/// path or the short one a document wrote, and a runtime component by the
/// name it was declared under.
///
/// Allowing narrows an unset filter to what is allowed and takes a type off a
/// denylist; denying does the reverse:
///
/// ```
/// # use beet_core::prelude::*;
/// let filter = TypeFilter::deny_all().allow::<Name>();
/// filter.passes_type::<Name>().xpect_true();
/// filter.passes_type::<ChildOf>().xpect_false();
/// TypeFilter::default()
/// 	.deny::<Name>()
/// 	.passes(&ReflectName::of::<Name>())
/// 	.xpect_false();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum TypeFilter {
	/// Every type passes.
	#[default]
	All,
	/// Only these types pass.
	Allow(HashSet<ReflectName>),
	/// Every type but these passes.
	Deny(HashSet<ReflectName>),
}

impl TypeFilter {
	/// Every type passes.
	pub fn allow_all() -> Self { Self::All }

	/// No type passes until one is allowed.
	pub fn deny_all() -> Self { Self::Allow(default()) }

	/// Allow `T`.
	#[must_use]
	pub fn allow<T: ?Sized + TypePath>(self) -> Self {
		self.allow_name(ReflectName::of::<T>())
	}

	/// Deny `T`.
	#[must_use]
	pub fn deny<T: ?Sized + TypePath>(self) -> Self {
		self.deny_name(ReflectName::of::<T>())
	}

	/// Allow the type `name` names.
	#[must_use]
	pub fn allow_name(mut self, name: impl Into<ReflectName>) -> Self {
		let name = name.into();
		match &mut self {
			Self::All => self = Self::Allow([name].into_iter().collect()),
			Self::Allow(names) => {
				names.insert(name);
			}
			Self::Deny(names) => {
				names.remove(&name);
			}
		}
		self
	}

	/// Deny the type `name` names.
	#[must_use]
	pub fn deny_name(mut self, name: impl Into<ReflectName>) -> Self {
		let name = name.into();
		match &mut self {
			Self::All => self = Self::Deny([name].into_iter().collect()),
			Self::Allow(names) => {
				names.remove(&name);
			}
			Self::Deny(names) => {
				names.insert(name);
			}
		}
		self
	}

	/// Whether the type or runtime component `name` names passes.
	pub fn passes(&self, name: &ReflectName) -> bool {
		self.decide(|names| names.contains(name))
	}

	/// Whether `T` passes.
	pub fn passes_type<T: ?Sized + TypePath>(&self) -> bool {
		self.passes(&ReflectName::of::<T>())
	}

	/// Whether the reflected type at `path` passes, by its full path or its
	/// short one.
	pub fn passes_path(&self, path: &TypePathTable) -> bool {
		self.decide(|names| {
			names.contains(&ReflectName::new(path.path()))
				|| names.contains(&ReflectName::new(path.short_path()))
		})
	}

	/// Whether a type passes, `listed` saying whether a set names it.
	fn decide(&self, listed: impl Fn(&HashSet<ReflectName>) -> bool) -> bool {
		match self {
			Self::All => true,
			Self::Allow(names) => listed(names),
			Self::Deny(names) => !listed(names),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Allowing and denying take each other back, and a set names a type by
	/// either of its paths.
	#[crate::test]
	fn allow_and_deny() {
		TypeFilter::deny_all()
			.allow::<Name>()
			.deny::<Name>()
			.passes_type::<Name>()
			.xpect_false();
		let filter = TypeFilter::default().deny::<Name>().allow::<Name>();
		filter.passes_type::<Name>().xpect_true();
		filter.passes_type::<ChildOf>().xpect_true();
		TypeFilter::default()
			.deny_name("Name")
			.passes_path(
				<Name as bevy::reflect::Typed>::type_info().type_path_table(),
			)
			.xpect_false();
	}
}
