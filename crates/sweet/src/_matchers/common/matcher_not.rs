#[extend::ext(name=SweetNot)]
pub impl<T> T {
	fn xpect_not<'a>(&'a self) -> MaybeNot<&'a T> { MaybeNot::Negated(self) }
}


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
}

impl<T> std::ops::Deref for MaybeNot<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		match self {
			MaybeNot::Negated(value) => value,
			MaybeNot::Verbatim(value) => value,
		}
	}
}

impl<T> From<T> for MaybeNot<T> {
	fn from(value: T) -> Self { MaybeNot::Verbatim(value) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;

	fn assert_true<'a>(val: impl Into<MaybeNot<&'a bool>>) -> Result<()> {
		let val = val.into();
		let result = val.inner();
		match (result, val.is_negated()) {
			(true, false) => Ok(()),
			(false, true) => Ok(()),
			_ => anyhow::bail!("failed"),
		}
	}

	#[test]
	fn works() {
		assert_true(&true).xpect().to_be_ok();
		assert_true(false.xpect_not()).xpect().to_be_ok();
	}
	#[test]
	fn errors() {
		assert_true(&false).xpect().to_be_err();
		assert_true(true.xpect_not()).xpect().to_be_err();
	}
}
