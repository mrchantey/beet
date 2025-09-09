#[extend::ext(name=SweetNot)]
pub impl<T> T {
	fn xnot(self) -> MaybeNot<T> { MaybeNot::Negated(self) }
}

#[derive(Debug, Copy, Clone)]
pub enum MaybeNot<T> {
	Negated(T),
	Verbatim(T),
}

impl<T> std::fmt::Display for MaybeNot<T>
where
	T: std::fmt::Display,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MaybeNot::Negated(value) => write!(f, "NOT {}", value),
			MaybeNot::Verbatim(value) => write!(f, "{}", value),
		}
	}
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
	pub fn passes_with_message(
		&self,
		result: bool,
		message: impl AsRef<str>,
	) -> Result<(), String> {
		match (result, self.is_negated()) {
			(true, false) => Ok(()),
			(false, true) => Ok(()),
			(true, true) => Err(format!("NOT {}", message.as_ref())),
			(false, false) => Err(message.as_ref().to_string()),
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

pub trait IntoMaybeNot<T>: Sized {
	fn into_maybe_not(self) -> MaybeNot<T>;
}
impl<T> IntoMaybeNot<T> for T {
	fn into_maybe_not(self) -> MaybeNot<T> { MaybeNot::Verbatim(self) }
}
impl<T> IntoMaybeNot<T> for MaybeNot<T> {
	fn into_maybe_not(self) -> MaybeNot<T> { self }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;

	trait Foo<T>: IntoMaybeNot<T>
	where
		T: PartialEq + std::fmt::Debug,
	{
		fn check(self, expected: T) -> Result<(), String> {
			self.into_maybe_not().compare_debug(&expected)
		}
	}
	impl<T, U> Foo<U> for T
	where
		T: IntoMaybeNot<U>,
		U: PartialEq + std::fmt::Debug,
	{
	}

	#[test]
	fn test_check() {
		true.check(true).xpect().to_be_ok();
		true.check(false).xpect().to_be_err();
		true.xnot().check(false).xpect().to_be_ok();
		true.xnot().check(true).xpect().to_be_err();
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
