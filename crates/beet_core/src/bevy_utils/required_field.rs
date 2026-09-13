//! [`RequiredField`]: the mark saying a reflected field must be authored.
use crate::prelude::*;

/// Reflect attribute marking a field the author must supply, so a partial
/// patch cannot silently leave it at the type's default.
///
/// A markup spread (`{Foo{..}}`) builds its component `from_reflect` over
/// `Default`, filling every field the patch omits; a field carrying this mark
/// is verified present first, and an absent one is a build error naming the
/// type and field.
///
/// Not a params concern: a request params type reads its required set from
/// its field types (anything that is not a `bool`, `Option` or `Vec`), see
/// [`MultiMapReflectExt`].
///
/// ```
/// # use beet_core::prelude::*;
/// #[derive(Default, Component, Reflect)]
/// #[reflect(Component, Default)]
/// struct EnsureSecret {
/// 	#[reflect(@RequiredField)]
/// 	secret: String,
/// }
/// ```
#[derive(Debug, Copy, Clone, Reflect)]
pub struct RequiredField;
