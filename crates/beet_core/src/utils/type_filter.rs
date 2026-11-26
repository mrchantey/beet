//! [`TypeFilter`]: a [`GlobFilter`] over types.
use crate::prelude::*;
use bevy::reflect::TypePath;
use bevy::reflect::TypePathTable;

/// A [`GlobFilter`] over types, each tested by every name it has: its full
/// path and its short path, or a runtime component's declared name. The last
/// rule matching any of them decides, so a rule reaches a type however it is
/// spelled, `"Name"` and `"bevy_ecs::name::Name"` alike, and an exclude cannot
/// be dodged by spelling the type the other way.
///
/// Authored as a [`GlobFilter`] is, the allowlist a human writes
/// (`read=["guestbook.*","Text"]`) or the two named lists; built in code from
/// the types themselves, each a literal of its full path:
///
/// ```
/// # use beet_core::prelude::*;
/// let filter = TypeFilter::deny_all().allow::<Name>();
/// filter.passes_type::<Name>().xpect_true();
/// filter.passes_type::<ChildOf>().xpect_false();
/// // a short pattern reaches the type by its short name
/// let filter = TypeFilter::default().with_exclude("Name");
/// filter.passes(&TypeName::new("bevy_ecs::name::Name")).xpect_false();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct TypeFilter(GlobFilter);

impl TypeFilter {
	/// A filter of these rules.
	pub fn new(filter: GlobFilter) -> Self { Self(filter) }

	/// Every type passes, the empty filter.
	pub fn allow_all() -> Self { Self::default() }

	/// No type passes until one is allowed after this.
	pub fn deny_all() -> Self { Self(GlobFilter::default().with_exclude("*")) }

	/// The rules.
	pub fn filter(&self) -> &GlobFilter { &self.0 }

	/// Whether there are no rules, so every type passes.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }

	/// Allow `T`, ahead of every rule before this.
	pub fn allow<T: ?Sized + TypePath>(mut self) -> Self {
		self.0
			.push(GlobRule::Include(GlobPattern::literal(T::type_path())));
		self
	}

	/// Deny `T`, ahead of every rule before this.
	pub fn deny<T: ?Sized + TypePath>(mut self) -> Self {
		self.0
			.push(GlobRule::Exclude(GlobPattern::literal(T::type_path())));
		self
	}

	/// Allow every type a name of which matches `pattern`.
	pub fn with_include(mut self, pattern: &str) -> Self {
		self.0.include(pattern);
		self
	}

	/// Deny every type a name of which matches `pattern`.
	pub fn with_exclude(mut self, pattern: &str) -> Self {
		self.0.exclude(pattern);
		self
	}

	/// Whether the type `name` names passes, by its full path or its short
	/// one.
	pub fn passes(&self, name: &TypeName) -> bool {
		self.is_empty() || self.0.passes_any([name.as_str(), &name.short()])
	}

	/// Whether `T` passes.
	pub fn passes_type<T: ?Sized + TypePath>(&self) -> bool {
		self.passes(&TypeName::of_path::<T>())
	}

	/// Whether the reflected type at `path` passes, by its full path or its
	/// short one.
	pub fn passes_path(&self, path: &TypePathTable) -> bool {
		self.is_empty() || self.0.passes_any([path.path(), path.short_path()])
	}
}

impl core::fmt::Display for TypeFilter {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Reflect)]
	struct Wrap<T>(T);

	/// A rule by either name reaches the type, and the rules keep their order.
	#[crate::test]
	fn any_name_last_match() {
		let filter = TypeFilter::default().with_exclude("Name");
		filter.passes_type::<Name>().xpect_false();
		filter
			.passes(&TypeName::new("bevy_ecs::name::Name"))
			.xpect_false();
		filter.passes_type::<ChildOf>().xpect_true();
		// a later rule takes an earlier one back
		TypeFilter::deny_all()
			.allow::<Name>()
			.passes_type::<Name>()
			.xpect_true();
		TypeFilter::default()
			.allow::<Name>()
			.deny::<Name>()
			.passes_type::<Name>()
			.xpect_false();
	}

	/// A generic type's path holds glob characters, matched literally.
	#[crate::test]
	fn generic_paths_are_literal() {
		let filter = TypeFilter::deny_all().allow::<Wrap<[u8; 4]>>();
		filter.passes_type::<Wrap<[u8; 4]>>().xpect_true();
		filter.passes_type::<Wrap<[u8; 3]>>().xpect_false();
	}
}
