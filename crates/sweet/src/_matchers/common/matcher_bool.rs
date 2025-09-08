use super::*;

impl Matcher<bool> {
	pub fn to_be_true(&self) { self.assert_equal(&true); }
	pub fn to_be_false(&self) { self.assert_equal(&false); }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn bool() {
		// true.xpect().to_be_false();
		true.xpect().to_be_true();
		false.xpect().not().to_be_true();

		false.xpect().to_be_false();
		true.xpect().not().to_be_false();
	}
}
