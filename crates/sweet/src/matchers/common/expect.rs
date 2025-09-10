use crate::prelude::*;
/// Create a new [`Matcher`] with the provided received value via the chain-only API.
///
/// # Example
///
/// ```rust
/// # use sweet::prelude::*;
/// true.xpect_true();
/// "foobar".xnot().xpect_starts_with("bar");
/// ```



pub trait Xpect: Sized {
	/// Create a new [`Matcher`] with the provided received value.
	fn xpect(self) -> Matcher<Self> { Matcher::new(self) }
}

impl<T> Xpect for T {}
