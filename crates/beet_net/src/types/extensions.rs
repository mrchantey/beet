//! Typed values a request carries from one handler to the next.
use alloc::boxed::Box;
use beet_core::prelude::*;
use core::any::Any;
use core::any::TypeId;

/// Values a [`Request`](crate::prelude::Request) carries between the
/// handlers it threads through in one process, one per type, ie the dry
/// repos every step of a dry run writes to
/// ([`DryRunRepos`](crate::prelude::DryRunRepos)).
///
/// - **State a handler made, never config.** A value a caller could set is a
///   param, declared by the extractor that takes it so `--help` documents
///   it, and an instruction every transport honours (ie
///   [`Redirects`](crate::prelude::Redirects)) is a field of the request
///   itself. An extension's type is one the request knows nothing about.
/// - **Never sent.** A transport reads only the parts and the body, so a
///   request forwarded to another process arrives without its extensions.
/// - **Dropped by [`into_parts`](crate::prelude::Request::into_parts).** A
///   handler that takes the request apart and rebuilds it carries them over
///   itself.
#[derive(Default)]
pub struct Extensions(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

impl core::fmt::Debug for Extensions {
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		formatter
			.debug_struct("Extensions")
			.field("len", &self.0.len())
			.finish()
	}
}

impl Extensions {
	/// The `T` carried, if any.
	pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
		self.0
			.get(&TypeId::of::<T>())
			.and_then(|value| value.downcast_ref())
	}

	/// The `T` carried, a default one inserted first when there is none.
	pub fn get_or_default<T: 'static + Send + Sync + Default>(
		&mut self,
	) -> &mut T {
		self.0
			.entry(TypeId::of::<T>())
			.or_insert_with(|| Box::new(T::default()))
			.downcast_mut()
			.expect("an extension is keyed by its own type id")
	}

	/// Carry `value`, answering the `T` it replaces.
	pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) -> Option<T> {
		self.0
			.insert(TypeId::of::<T>(), Box::new(value))
			.and_then(|old| old.downcast().ok())
			.map(|old| *old)
	}
}
