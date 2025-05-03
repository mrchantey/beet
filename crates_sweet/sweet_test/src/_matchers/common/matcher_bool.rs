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
		expect(true).to_be_true();
		expect(false).not().to_be_true();

		expect(false).to_be_false();
		expect(true).not().to_be_false();
	}
}
