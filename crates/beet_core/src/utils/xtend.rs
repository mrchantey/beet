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
/// # use beet_core::prelude::*;
/// # use bevy::prelude::*;
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
pub trait Xtend: Sized {
	/// Applies a function to `self` and returns the result.
	///
	/// Similar to [`Iterator::map`] but works on any type, enabling method chaining.
	fn xmap<O>(self, func: impl FnOnce(Self) -> O) -> O { func(self) }

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
		Self: std::fmt::Debug,
	{
		crate::cross_log!("{}: {:#?}", prefix.as_ref(), self);
		self
	}

	/// Prints the display-formatted value and returns `self`.
	fn xprint_display(self) -> Self
	where
		Self: std::fmt::Display,
	{
		crate::cross_log!("{}", self);
		self
	}

	/// Prints the debug-formatted value and returns `self`.
	fn xprint_debug(self) -> Self
	where
		Self: std::fmt::Debug,
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
	fn xok<E>(self) -> Result<Self, E> { Ok(self) }

	/// Wraps `self` in [`Some`].
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// assert_eq!("foo".xsome(), Some("foo"));
	/// ```
	fn xsome(self) -> Option<Self> { Some(self) }

	/// Converts `self` using [`Into::into`].
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// assert_eq!(7_u32.xinto::<u64>(), 7);
	/// ```
	fn xinto<T: From<Self>>(self) -> T { T::from(self) }

	/// Returns a pretty-printed `Debug` representation.
	fn xfmt(&self) -> String
	where
		Self: std::fmt::Debug,
	{
		format!("{:#?}", self)
	}

	/// Returns a compact `Debug` representation.
	fn xfmt_debug(&self) -> String
	where
		Self: std::fmt::Debug,
	{
		format!("{:?}", self)
	}

	/// Returns a `Display` representation.
	fn xfmt_display(&self) -> String
	where
		Self: std::fmt::Display,
	{
		format!("{}", self)
	}
}

impl<T: Sized> Xtend for T {}


/// Iterator-like operations that collect immediately into a [`Vec`].
///
/// These methods combine iteration and collection into single operations
/// for more concise code when you need a collected result anyway.
pub trait XtendIter<T>: Sized + IntoIterator<Item = T> {
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
#[extend::ext(name=XtendBool)]
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

impl<T: Sized, I: IntoIterator<Item = T>> XtendIter<T> for I {}


/// Chainable operations for vectors.
pub trait XtendVec<T> {
	/// Extends `self` with items from an iterator and returns `self`.
	fn xtend<I: IntoIterator<Item = T>>(self, iter: I) -> Self;

	/// Pushes an item and returns `self`.
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

/// Chainable operations for strings.
pub trait XtendString {
	/// Appends a string and returns `self`.
	fn xtend(self, item: impl AsRef<str>) -> Self;
}

impl XtendString for String {
	fn xtend(mut self, item: impl AsRef<str>) -> Self {
		self.push_str(item.as_ref());
		self
	}
}
