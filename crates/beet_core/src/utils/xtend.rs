//! Method-chaining utilities for any type.
//!
//! This module provides extension traits that enable fluent method chaining
//! on any type, similar to the [`tap`](https://crates.io/crates/tap) crate.
//!
//! # Overview
//!
//! - [`Xtend`] - Core utilities for mapping, tapping, and wrapping any type
//! - [`XtendIter`] - Iterator-like operations that collect immediately
//! - [`XtendVec`] - Chainable vector operations
//! - [`XtendString`] - Chainable string operations
//! - [`XtendBool`] - Boolean-conditional mapping

use crate::prelude::*;
use bevy::prelude::*;

/// Method-chaining utilities for any type.
///
/// This trait provides functional-style operations that enable fluent method
/// chains without breaking the flow for common operations like wrapping in
/// `Result` or `Option`, or applying transformations.
///
/// # Examples
///
/// Mapping inline:
///
/// ```
/// # use beet_core::prelude::*;
/// let doubled = 5.xmap(|n| n * 2);
/// assert_eq!(doubled, 10);
/// ```
///
/// Wrapping in Result:
///
/// ```
/// # use bevy::prelude::*;
/// # use beet_core::prelude::*;
/// fn compute() -> Result<i32> {
///     42.xok()
/// }
/// ```
///
/// Tapping for side effects:
///
/// ```
/// # use beet_core::prelude::*;
/// let value = vec![1, 2, 3]
///     .xtap(|v| assert_eq!(v.len(), 3))
///     .into_iter()
///     .sum::<i32>();
/// assert_eq!(value, 6);
/// ```
#[extend::ext(name = Xtend)]
pub impl<T: Sized> T {
	/// Applies a function to `self` and returns the result.
	///
	/// Similar to [`Iterator::map`] but works on any type, enabling method chaining.
	fn xmap<O>(self, func: impl FnOnce(Self) -> O) -> O
	where
		Self: Sized,
	{
		func(self)
	}

	/// Applies a function to `&mut self` for side effects, then returns `self`.
	///
	/// Similar to [`Iterator::inspect`] but works on any type.
	fn xtap(mut self, func: impl FnOnce(&mut Self)) -> Self {
		func(&mut self);
		self
	}

	/// Prints the debug-formatted value with a prefix and returns `self`.
	///
	/// Uses [`cross_log!`](crate::cross_log) for cross-platform output.
	fn xprint(self, prefix: impl AsRef<str>) -> Self
	where
		Self: core::fmt::Debug,
	{
		crate::cross_log!("{}: {:#?}", prefix.as_ref(), self);
		self
	}

	/// Prints the display-formatted value and returns `self`.
	fn xprint_display(self) -> Self
	where
		Self: core::fmt::Display,
	{
		crate::cross_log!("{}", self);
		self
	}

	/// Prints the debug-formatted value and returns `self`.
	fn xprint_debug(self) -> Self
	where
		Self: core::fmt::Debug,
	{
		crate::cross_log!("{:?}", self);
		self
	}

	/// Applies a function to `&mut self` for side effects, then returns `&mut self`.
	fn xtap_mut(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
		func(self);
		self
	}

	/// Returns a reference to `self` for use in method chains.
	fn xref(&self) -> &Self { self }

	/// Returns a mutable reference to `self` for use in method chains.
	fn xmut(&mut self) -> &mut Self { self }

	/// Wraps `self` in [`Ok`].
	///
	/// # Examples
	///
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_core::prelude::*;
	/// fn foo() -> Result<u32> {
	///     7.xok()
	/// }
	/// ```
	fn xok<E>(self) -> Result<Self, E>
	where
		Self: Sized,
	{
		Ok(self)
	}

	/// Wraps `self` in [`Err`].
	///
	/// # Examples
	///
	/// ```
	/// # use bevy::prelude::*;
	/// # use beet_core::prelude::*;
	/// fn foo() -> Result<u32, &'static str> {
	///     "error".xerr()
	/// }
	/// ```
	fn xerr<V>(self) -> Result<V, Self>
	where
		Self: Sized,
	{
		Err(self)
	}

	/// Wraps `self` in [`Some`].
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// assert_eq!("foo".xsome(), Some("foo"));
	/// ```
	fn xsome(self) -> Option<Self>
	where
		Self: Sized,
	{
		Some(self)
	}

	/// Wraps `self` in a `Vec`.
	fn xvec(self) -> Vec<Self>
	where
		Self: Sized,
	{
		let mut vec = Vec::with_capacity(1);
		vec.push(self);
		vec
	}

	/// Converts `self` using [`Into::into`].
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// assert_eq!(7_u32.xinto::<u64>(), 7);
	/// ```
	fn xinto<U: From<Self>>(self) -> U
	where
		Self: Sized,
	{
		U::from(self)
	}

	/// Returns a pretty-printed `Debug` representation.
	fn xfmt(&self) -> String
	where
		Self: core::fmt::Debug,
	{
		format!("{:#?}", self)
	}

	/// Returns a compact `Debug` representation.
	fn xfmt_debug(&self) -> String
	where
		Self: core::fmt::Debug,
	{
		format!("{:?}", self)
	}

	/// Returns a `Display` representation.
	fn xfmt_display(&self) -> String
	where
		Self: core::fmt::Display,
	{
		format!("{}", self)
	}
}

/// Iterator-like operations that collect immediately into a [`Vec`].
///
/// These methods combine iteration and collection into single operations
/// for more concise code when you need a collected result anyway.
#[extend::ext(name = XtendIter)]
pub impl<T, I: IntoIterator<Item = T>> I {
	/// Adds the provided item to the end of the iterator,
	/// using std::iter::once, returning another iterator.
	fn xpush(self, item: T) -> impl IntoIterator<Item = T> {
		self.into_iter().chain(std::iter::once(item))
	}

	/// Maps each item and collects into a [`Vec`].
	///
	/// Equivalent to `.into_iter().map(func).collect()`.
	fn xmap_each<O>(self, func: impl FnMut(T) -> O) -> Vec<O> {
		self.into_iter().map(func).collect()
	}

	/// Maps each item with a fallible function, short-circuiting on error.
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

	/// Like map but async.
	fn xmap_async<O, E>(
		self,
		mut func: impl AsyncFnMut(T) -> Result<O, E>,
	) -> impl Future<Output = Result<Vec<O>, E>> {
		async move {
			let mut out = Vec::new();
			for item in self.into_iter() {
				match (func)(item).await {
					Ok(o) => out.push(o),
					Err(e) => return Err(e),
				}
			}
			Ok(out)
		}
	}

	/// Filters each item with a fallible function, short-circuiting on error.
	fn xtry_filter<E>(
		self,
		mut func: impl FnMut(&mut T) -> Result<bool, E>,
	) -> Result<Vec<T>, E> {
		let mut out = Vec::new();
		for mut item in self.into_iter() {
			match (func)(&mut item) {
				Ok(true) => out.push(item),
				Ok(false) => {}
				Err(e) => return Err(e),
			}
		}
		Ok(out)
	}

	/// Filter-maps each item with a fallible function, short-circuiting on error.
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

	/// Finds the first successful result or returns the first error.
	///
	/// Similar to [`Iterator::find_map`] but for fallible operations.
	/// Returns `Ok` on the first successful result, otherwise returns
	/// the first error encountered (or a "No items found" error if all
	/// returned `None`-equivalent values).
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

/// Extension methods for boolean values.
#[extend::ext(name = XtendBool)]
pub impl bool {
	/// Runs the function if `self` is true, returning `Some(result)`.
	fn xmap_true<O>(&self, func: impl FnOnce() -> O) -> Option<O> {
		if *self { Some(func()) } else { None }
	}

	/// Runs the function if `self` is false, returning `Some(result)`.
	fn xmap_false<O>(&self, func: impl FnOnce() -> O) -> Option<O> {
		if !*self { Some(func()) } else { None }
	}
}

/// Chainable operations for vectors.
#[extend::ext(name = XtendVec)]
pub impl<T, T2: AsMut<Vec<T>>> T2 {
	/// Extends `self` with items from an iterator and returns `self`.
	fn xtend<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
		self.as_mut().extend(iter);
		self
	}

	/// Pushes an item and returns `self`.
	fn xpush(mut self, item: T) -> Self {
		self.as_mut().push(item);
		self
	}
}

/// Chainable operations for strings.
#[extend::ext(name = XtendString)]
pub impl<T: Into<String>> T {
	/// Appends a string and returns `self`.
	fn xtend(self, item: impl AsRef<str>) -> String {
		let mut this = self.into();
		this.push_str(item.as_ref());
		this
	}
}

/// Extension trait for types that can be converted into an iterator.
///
/// Uses a marker type `M` to allow two blanket impls to coexist: one for
/// existing iterators and one for wrapping a single value.
pub trait XIntoIterator<M, T> {
	/// Converts `self` into an iterator of items of type `T`.
	fn xinto_iter(self) -> impl Iterator<Item = T>;
}

/// Marker type for the [`IntoIterator`] implementation of [`XIntoIterator`].
pub struct IteratorIntoIteratorMarker;

impl<T, I> XIntoIterator<IteratorIntoIteratorMarker, T> for I
where
	I: IntoIterator<Item = T>,
{
	fn xinto_iter(self) -> impl Iterator<Item = T> { self.into_iter() }
}

impl<T> XIntoIterator<Self, T> for T {
	fn xinto_iter(self) -> impl Iterator<Item = T> { [self].into_iter() }
}
