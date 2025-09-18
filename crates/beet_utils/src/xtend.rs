/// Basically a `FnOnce` trait, but not nightly and a little less awkward to implement.
pub trait Pipeline<In, Out = In> {
	/// Consume self and apply to the target
	fn apply(self, value: In) -> Out;
}

impl<F, In, Out> Pipeline<In, Out> for F
where
	F: FnOnce(In) -> Out,
{
	fn apply(self, value: In) -> Out { self(value) }
}


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
	/// just print the value and return it
	fn xprint(self) -> Self
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
	/// just print the value and return it, debug formatted
	fn xprint_fmtdebug(self) -> Self
	where
		Self: std::fmt::Debug,
	{
		println!("{:#?}", self);
		self
	}
	/// Similar to [`Iterator::inspect`] but for any type, not just iterators, and mutable.
	fn xtap_mut(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
		func(self);
		self
	}
	/// Similar to [`Iterator::map`] but for any type, not just iterators,
	/// using a custom [`Pipeline`] trait which behaves similarly to a `FnOnce` trait,
	/// but available on stable rust.
	fn xpipe<P: Pipeline<Self, O>, O>(self, pipeline: P) -> O {
		pipeline.apply(self)
	}

	/// Convenience wrapper for `&self` in method chaining contexts.
	fn xref(&self) -> &Self { self }
	/// Convenience wrapper for `&mut self` in method chaining contexts.
	fn xmut(&mut self) -> &mut Self { self }
	/// Wraps the value in a [`Result::Ok`]
	///
	/// ## Example
	///
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// assert_eq!("foo".xok::<()>(), Ok("foo"));
	/// ```
	fn xok<E>(self) -> Result<Self, E> { Ok(self) }
	/// Wraps the value in an [`Option::Some`]
	/// ## Example
	///
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// assert_eq!("foo".xsome(), Some("foo"));
	/// ```
	fn xsome(self) -> Option<Self> { Some(self) }

	/// Convenience wrapper for [`Into::into`].
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// assert_eq!(7_u32.xinto::<u64>(), 7);
	/// ```
	fn xinto<T: From<Self>>(self) -> T { T::from(self) }

	/// Return a `String` containing the `Debug` representation of the value.
	///
	/// Unlike `xdebug` which prints the value to stdout and returns the value,
	/// this method returns the formatted debug string.
	fn xfmt_debug(&self) -> String
	where
		Self: std::fmt::Debug,
	{
		format!("{:?}", self)
	}

	/// Return a `String` containing the `Display` representation of the value.
	///
	/// Similar to `xfmt_debug`, but uses the `Display` formatting instead of `Debug`.
	fn xfmt(&self) -> String
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
}

impl<T: Sized, I: IntoIterator<Item = T>> XtendIter<T> for I {}

pub trait XtendVec<T> {
	/// Similar to [`Vec::extend`] but returns [`Self`]
	fn xtend<I: IntoIterator<Item = T>>(self, iter: I) -> Self;
}

impl<T, T2> XtendVec<T> for T2
where
	T2: AsMut<Vec<T>>,
{
	fn xtend<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
		self.as_mut().extend(iter);
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
