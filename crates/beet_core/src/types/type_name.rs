use crate::prelude::*;
use core::any::TypeId;
use core::ops::Deref;
use smol_str::ToSmolStr;

/// The full path of a rust type, ie `alloc::string::String`, the one way beet
/// names a type: in diagnostics, in param declarations, and through
/// [`short`](Self::short) as the tag markup and the reflect registry author it
/// under.
///
/// Identity is the full path, so `a::Foo` and `b::Foo` are distinct names that
/// share the short name `Foo`. A name a document authors may be short, as a
/// tag is: [`registration`](Self::registration) resolves either against a
/// registry, and [`names`](Self::names) asks whether one names a type.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let name = TypeName::of::<Option<Vec<usize>>>();
/// name.as_str().xpect_eq("core::option::Option<alloc::vec::Vec<usize>>");
/// name.short().xpect_eq("Option<Vec<usize>>");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct TypeName(SmolStr);

impl TypeName {
	/// The name of `T`, from [`core::any::type_name`].
	pub fn of<T: ?Sized>() -> Self {
		Self(SmolStr::new_static(core::any::type_name::<T>()))
	}

	/// The name of `T` as reflection knows it, which a registry resolves: its
	/// [`TypePath`](bevy::reflect::TypePath), equal to [`of`](Self::of) for
	/// a type declared in a module and the module path where a function
	/// declares it.
	pub fn of_path<T: ?Sized + bevy::reflect::TypePath>() -> Self {
		Self::new(T::type_path())
	}

	/// The name of the type of `val`.
	pub fn of_val<T: ?Sized>(_val: &T) -> Self { Self::of::<T>() }

	/// A name from its type path, ie a reflected field's `type_path()`, full
	/// wherever it is known.
	pub fn new(type_path: impl Into<SmolStr>) -> Self { Self(type_path.into()) }

	/// The registration this names in `registry`, by full path or, as markup
	/// writes a tag, by short path.
	pub fn registration<'a>(
		&self,
		registry: &'a bevy::reflect::TypeRegistry,
	) -> Option<&'a bevy::reflect::TypeRegistration> {
		registry
			.get_with_type_path(self.as_str())
			.or_else(|| reflect_ext::registration_by_name(registry, self))
			// a path only `core::any::type_name` writes, ie of a type a
			// function declares, still names it by its short path
			.or_else(|| {
				reflect_ext::registration_by_name(registry, &self.short())
			})
	}

	/// Whether this names the type at `type_path`: its full path, or the short
	/// path of it a document authored.
	pub fn names(&self, type_path: &bevy::reflect::TypePathTable) -> bool {
		self.as_str() == type_path.path()
			|| self.as_str() == type_path.short_path()
	}

	/// The full type path.
	pub fn as_str(&self) -> &str { self.0.as_str() }

	/// The short name, every path segment shortened, generics included:
	/// `Option<Vec<usize>>` for `core::option::Option<alloc::vec::Vec<usize>>`.
	///
	/// This is the reflect `short_type_path` a template or component is
	/// registered and authored under, so a rust caller says
	/// `TypeName::of::<ArticleLayout>().short()` where a document says
	/// `<ArticleLayout/>`, a symbol the compiler checks and a rename follows.
	pub fn short(&self) -> SmolStr { ShortName(&self.0).to_smolstr() }
}

impl core::fmt::Display for TypeName {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.0)
	}
}

impl Deref for TypeName {
	type Target = str;
	fn deref(&self) -> &str { self.0.as_str() }
}

impl AsRef<str> for TypeName {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl From<TypeName> for SmolStr {
	fn from(name: TypeName) -> Self { name.0 }
}

impl From<&str> for TypeName {
	fn from(type_path: &str) -> Self { Self::new(type_path) }
}

impl From<SmolStr> for TypeName {
	fn from(type_path: SmolStr) -> Self { Self(type_path) }
}

/// A [`TypeName`] paired with its [`TypeId`]: the id for comparison, the name
/// for display.
#[derive(Debug, Clone)]
pub struct TypeMeta {
	type_name: TypeName,
	type_id: TypeId,
}

impl TypeMeta {
	/// The [`TypeMeta`] of `T`.
	pub fn of<T: 'static + ?Sized>() -> Self {
		Self {
			type_name: TypeName::of::<T>(),
			type_id: TypeId::of::<T>(),
		}
	}

	/// The [`TypeMeta`] of the type of `val`.
	pub fn of_val<T: 'static + ?Sized>(_val: &T) -> Self { Self::of::<T>() }

	/// The type's name, ie `core::option::Option<i32>`.
	pub fn type_name(&self) -> &TypeName { &self.type_name }

	/// The type's [`TypeId`].
	pub fn type_id(&self) -> TypeId { self.type_id }

	/// Whether this is the meta of `T`.
	pub fn is<T: 'static + ?Sized>(&self) -> bool {
		self.type_id == TypeId::of::<T>()
	}
}

impl core::fmt::Display for TypeMeta {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.type_name.fmt(f)
	}
}

/// Identity is the [`TypeId`] alone; the name is display data derived from it.
impl PartialEq for TypeMeta {
	fn eq(&self, other: &Self) -> bool { self.type_id == other.type_id }
}
impl Eq for TypeMeta {}

impl core::hash::Hash for TypeMeta {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.type_id.hash(state);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::reflect::TypePath;

	#[derive(Reflect)]
	struct Widget<T>(T);

	#[crate::test]
	fn short_matches_reflect_short_path() {
		fn check<T: TypePath>() {
			TypeName::of::<T>().short().xpect_eq(T::short_type_path());
		}
		check::<Widget<u32>>();
		check::<Widget<Vec<String>>>();
		check::<Option<Vec<usize>>>();
		check::<String>();
	}

	#[crate::test]
	fn identity_is_the_full_path() {
		mod other {
			pub struct Widget;
		}
		struct Widget;
		TypeName::of::<Widget>()
			.short()
			.xpect_eq(TypeName::of::<other::Widget>().short());
		TypeName::of::<Widget>().xpect_not_eq(TypeName::of::<other::Widget>());
	}
}
