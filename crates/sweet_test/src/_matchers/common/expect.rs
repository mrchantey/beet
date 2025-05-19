use crate::prelude::*;
/// Create a new [`Matcher`] with the provided received value.
///
/// # Example
///
/// ```rust
/// # use sweet_test::prelude::*;
/// expect(true).to_be_true();
/// expect("foobar").not().to_start_with("bar");
/// ```
pub fn expect<T>(value: T) -> Matcher<T> { Matcher::new(value) }



pub trait Xpect: Sized {
	/// Create a new [`Matcher`] with the provided received value.
	fn xpect(self) -> Matcher<Self> { Matcher::new(self) }
}

impl<T> Xpect for T {}
