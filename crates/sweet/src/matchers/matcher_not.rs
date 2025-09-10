use std::fmt::Display;

#[extend::ext(name=SweetNot)]
pub impl<T> T {
	fn xnot(self) -> MaybeNot<T> { MaybeNot::Negated(self) }
}

#[derive(Debug, Copy, Clone)]
pub enum MaybeNot<T> {
	Negated(T),
	Verbatim(T),
}

impl<T> MaybeNot<T> {
	pub fn is_negated(&self) -> bool {
		match self {
			MaybeNot::Negated(_) => true,
			MaybeNot::Verbatim(_) => false,
		}
	}
	pub fn inner(&self) -> &T {
		match self {
			MaybeNot::Negated(value) => value,
			MaybeNot::Verbatim(value) => value,
		}
	}
	pub fn into_inner(self) -> T {
		match self {
			MaybeNot::Negated(value) => value,
			MaybeNot::Verbatim(value) => value,
		}
	}

	/// Performs an equality check, considering if `self`
	/// is negated
	pub fn passes_display(
		&self,
		result: bool,
		expected: impl std::fmt::Display,
	) -> Result<(), String> {
		match (result, self.is_negated()) {
			(true, false) => Ok(()),
			(false, true) => Ok(()),
			(true, true) => Err(format!("NOT {}", expected)),
			(false, false) => Err(format!("{}", expected)),
		}
	}
	/// Performs an equality check, considering if `self`
	/// is negated
	pub fn passes_debug(
		&self,
		result: bool,
		expected: impl std::fmt::Debug,
	) -> Result<(), String> {
		match (result, self.is_negated()) {
			(true, false) => Ok(()),
			(false, true) => Ok(()),
			(true, true) => Err(format!("NOT {:?}", expected)),
			(false, false) => Err(format!("{:?}", expected)),
		}
	}
	/// Performs an equality check, considering if `self`
	/// is negated
	pub fn passes<T2>(&self, other: &T2) -> bool
	where
		T: PartialEq<T2>,
	{
		match self {
			MaybeNot::Negated(value) => value != other,
			MaybeNot::Verbatim(value) => value == other,
		}
	}

	pub fn compare_debug<Expected>(
		&self,
		expected: &Expected,
	) -> Result<(), String>
	where
		Expected: std::fmt::Debug,
		T: PartialEq<Expected>,
	{
		if self.passes(expected) {
			Ok(())
		} else {
			Err(format!("NOT {:?}", expected))
		}
	}
	pub fn compare_display<Expected>(
		&self,
		expected: &Expected,
	) -> Result<(), String>
	where
		Expected: std::fmt::Display,
		T: PartialEq<Expected>,
	{
		if self.passes(expected) {
			Ok(())
		} else {
			Err(format!("NOT {}", expected))
		}
	}
}
/// Blanket trait implemented for:
/// - All `Display` types
/// - `IntoMaybeNot<T:Display>`
/// Useful for performing assertions where expected and received
/// may be different types, like `&str` and `String`.
/// This approach is similar to the into `BevyError` blanket.
pub trait IntoMaybeNotDisplay<T>: Sized {
	fn into_maybe_not(self) -> MaybeNot<T>;
}
impl<T> IntoMaybeNotDisplay<T> for T
where
	T: Display,
{
	fn into_maybe_not(self) -> MaybeNot<T> { MaybeNot::Verbatim(self) }
}
impl<T> IntoMaybeNotDisplay<T> for MaybeNot<T>
where
	T: Display,
{
	fn into_maybe_not(self) -> MaybeNot<T> { self }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;

	#[extend::ext]
	impl<T, U> T
	where
		Self: IntoMaybeNotDisplay<U>,
		U: PartialEq + std::fmt::Debug,
	{
		fn check(self, expected: U) -> Result<(), String> {
			self.into_maybe_not().compare_debug(&expected)
		}
		// this only works because of MaybeNotDisplay,
		// otherwise multiple impls
		fn check_untyped<V>(self, expected: V) -> Result<(), String>
		where
			V: std::fmt::Debug,
			U: PartialEq<V>,
		{
			self.into_maybe_not().compare_debug(&expected)
		}
	}

	#[test]
	fn test_check() {
		true.check(true).xpect_ok();
		true.check(false).xpect_err();
		true.xnot().check(false).xpect_ok();
		true.xnot().check(true).xpect_err();

		MaybeNot::Negated(false).check(true).xpect_ok();
		MaybeNot::Negated(false).check_untyped(true).xpect_ok();
	}

	#[test]
	fn passes() {
		MaybeNot::Verbatim(true).passes(&true).xpect_true();
		MaybeNot::Verbatim(false).passes(&true).xpect_false();
	}

	#[test]
	fn into() {
		let _val: MaybeNot<_> = true.into_maybe_not();
		let _val: MaybeNot<bool> = MaybeNot::Negated(true).into();
	}
}
