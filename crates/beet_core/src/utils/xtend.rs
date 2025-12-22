use crate::prelude::*;
use bevy::prelude::*;

/// Utilities for method-chaining on any type.
/// Very similar in its goals to [`tap`](https://crates.io/crates/tap)
pub trait Xtend: Sized {
	/// Similar to [`Iterator::map`] but for any type, not just iterators,
	/// allowing for method chaining.
	fn xmap<O>(self, func: impl FnOnce(Self) -> O) -> O { func(self) }
	/// Similar to [`Iterator::inspect`] but for any type, not just iterators.
	fn xtap(mut self, func: impl FnOnce(&mut Self)) -> Self {
		func(&mut self);
		self
	}
	/// just print the value and return it, debug formatted
	fn xprint(self, prefix: impl AsRef<str>) -> Self
	where
		Self: std::fmt::Debug,
	{
		println!("{}: {:#?}", prefix.as_ref(), self);
		self
	}
	/// just print the value and return it
	fn xprint_display(self) -> Self
	where
		Self: std::fmt::Display,
	{
		println!("{}", self);
		self
	}
	/// just print the value and return it, debug
	fn xprint_debug(self) -> Self
	where
		Self: std::fmt::Debug,
	{
		println!("{:?}", self);
		self
	}
	/// Similar to [`Iterator::inspect`] but for any type, not just iterators, and mutable.
	fn xtap_mut(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
		func(self);
		self
	}

	/// Convenience wrapper for `&self` in method chaining contexts.
	fn xref(&self) -> &Self { self }
	/// Convenience wrapper for `&mut self` in method chaining contexts.
	fn xmut(&mut self) -> &mut Self { self }
	/// A message-chaining friendly way wrap this type in a [`Result::Ok`]
	///
	/// ## Example
	///
	/// ```rust
	/// # use bevy::prelude::*;
	/// # use beet_core::prelude::*;
	/// fn foo()-> Result<u32> {
	///   7.xok()
	/// }
	/// ```
	fn xok<E>(self) -> Result<Self, E> { Ok(self) }
	/// Wraps the value in an [`Option::Some`]
	/// ## Example
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// assert_eq!("foo".xsome(), Some("foo"));
	/// ```
	fn xsome(self) -> Option<Self> { Some(self) }

	/// Convenience wrapper for [`Into::into`].
	/// ```rust
	/// # use beet_core::prelude::*;
	/// assert_eq!(7_u32.xinto::<u64>(), 7);
	/// ```
	fn xinto<T: From<Self>>(self) -> T { T::from(self) }

	/// Return a `String` containing the formatted `Debug` representation of the value.
	fn xfmt(&self) -> String
	where
		Self: std::fmt::Debug,
	{
		format!("{:#?}", self)
	}
	/// Return a `String` containing the `Debug` representation of the value.
	fn xfmt_debug(&self) -> String
	where
		Self: std::fmt::Debug,
	{
		format!("{:?}", self)
	}


	/// Return a `String` containing the `Display` representation of the value.
	fn xfmt_display(&self) -> String
	where
		Self: std::fmt::Display,
	{
		format!("{}", self)
	}
}
impl<T: Sized> Xtend for T {}


/// Utilities for method-chaining on any type.
/// Very similar in its goals to [`tap`](https://crates.io/crates/tap)
pub trait XtendIter<T>: Sized + IntoIterator<Item = T> {
	/// Similar to [`IntoIterator::into_iter().map(func).collect()`]
	fn xmap_each<O>(self, func: impl FnMut(T) -> O) -> Vec<O> {
		self.into_iter().map(func).collect()
	}
	/// Similar to [`IntoIterator::into_iter().map(func).collect()`]
	/// but flattens the results.
	fn xtry_map<O, E>(
		self,
		mut func: impl FnMut(T) -> Result<O, E>,
	) -> Result<Vec<O>, E> {
		let mut out = Vec::new();
		for item in self.into_iter() {
			match (func)(item) {
				Ok(o) => out.push(o),
				Err(e) => return Err(e),
			}
		}
		Ok(out)
	}
	/// Similar to [`IntoIterator::into_iter().filter_map(func).collect()`]
	/// but flattens the results.
	fn xtry_filter_map<O, E>(
		self,
		mut func: impl FnMut(T) -> Result<Option<O>, E>,
	) -> Result<Vec<O>, E> {
		let mut out = Vec::new();
		for item in self.into_iter() {
			match (func)(item) {
				Ok(Some(o)) => out.push(o),
				Ok(None) => {}
				Err(e) => return Err(e),
			}
		}
		Ok(out)
	}

	/// Similar to [`Iterator::find_map`] but where the mapping can fail.
	///
	/// In the case of no Ok, the first error will be returned, otherwise
	/// a "No items found" is returned.
	fn xtry_find_map<O, E>(
		self,
		mut func: impl FnMut(T) -> Result<O, E>,
	) -> Result<O>
	where
		E: Into<BevyError>,
	{
		let mut err = None;
		for item in self.into_iter() {
			match (func)(item) {
				Ok(o) => return Ok(o),
				Err(e) if err.is_none() => err = Some(e),
				Err(_) => { /* ignore subsequent errors */ }
			}
		}
		match err {
			Some(e) => Err(e.into()),
			None => bevybail!("No items found"),
		}
	}
}

#[extend::ext(name=XtendBool)]
pub impl bool {
	/// Runs the function if `self` is true
	fn xmap_true<O>(&self, func: impl FnOnce() -> O) -> Option<O> {
		if *self { Some(func()) } else { None }
	}
	/// Runs the function if `self` is false
	fn xmap_false<O>(&self, func: impl FnOnce() -> O) -> Option<O> {
		if !*self { Some(func()) } else { None }
	}
}
impl<T: Sized, I: IntoIterator<Item = T>> XtendIter<T> for I {}


pub trait XtendVec<T> {
	/// Similar to [`Vec::extend`] but returns [`Self`]
	fn xtend<I: IntoIterator<Item = T>>(self, iter: I) -> Self;
	/// Similar to [`Vec::push`] but returns [`Self`]
	fn xpush(self, item: T) -> Self;
}

impl<T, T2> XtendVec<T> for T2
where
	T2: AsMut<Vec<T>>,
{
	fn xtend<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
		self.as_mut().extend(iter);
		self
	}

	fn xpush(mut self, item: T) -> Self {
		self.as_mut().push(item);
		self
	}
}

pub trait XtendString {
	/// Similar to [`String::push_str`] but returns [`Self`]
	fn xtend(self, item: impl AsRef<str>) -> Self;
}

impl XtendString for String {
	fn xtend(mut self, item: impl AsRef<str>) -> Self {
		self.push_str(item.as_ref());
		self
	}
}
