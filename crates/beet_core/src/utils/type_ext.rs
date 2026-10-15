//! Type-name utilities: the one place a rust type is named the way markup and
//! the reflect registry name it.

use crate::prelude::*;

/// The short type name of `T`, ie `ArticleLayout` for
/// `my_site::layouts::ArticleLayout`.
///
/// Each path segment is shortened, generics included, so this matches the
/// reflect `short_path` a template or component is registered and authored
/// under: `Option<Vec<usize>>`, never `core::option::Option<..>`.
///
/// The named-resolution counterpart of a markup tag: a rust caller says
/// `type_ext::short_name::<ArticleLayout>()` where a document says
/// `<ArticleLayout/>`, so the name is a symbol the compiler checks and a
/// rename follows, rather than a string.
pub fn short_name<T: ?Sized>() -> SmolStr {
	short_path(core::any::type_name::<T>())
}

/// [`short_name`] of a type by its full path, ie `Vec<String>` for
/// `alloc::vec::Vec<alloc::string::String>`.
pub fn short_path(type_path: &str) -> SmolStr {
	SmolStr::from(ShortName(type_path).to_string())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	struct Widget;

	#[crate::test]
	fn shortens_paths() {
		type_ext::short_name::<Widget>().xpect_eq("Widget");
		// every segment shortens, so a generic reads as its markup tag does
		type_ext::short_name::<Option<alloc::vec::Vec<usize>>>()
			.xpect_eq("Option<Vec<usize>>");
	}
}
