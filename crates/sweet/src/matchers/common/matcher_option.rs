use super::*;



impl<T> Matcher<Option<T>>
where
	T: std::fmt::Debug,
{
	pub fn to_be_option(&self, expected: bool) {
		if expected {
			let result = self.value.is_some();
			self.assert_correct(result, &"Some");
		} else {
			let result = self.value.is_none();
			self.assert_correct(result, &"None");
		}
	}
	pub fn to_be_some(&self) {
		let result = self.value.is_some();
		self.assert_correct(result, &"Some");
	}

	/// # Panics
	/// Panics if the value is `None`
	pub fn as_some(self) -> Matcher<T> {
		if let Some(value) = self.value {
			Matcher::new(value)
		} else {
			self.assert_with_expected_received(&"Some", &"None");
		}
	}
	pub fn to_be_none(&self) {
		let result = self.value.is_none();
		self.assert_correct(result, &"None");
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn option() {
		Some(true).xpect().to_be_some();
		Some(true).xpect().not().to_be_none();

		(None::<bool>).xpect().to_be_none();
		(None::<bool>).xpect().not().to_be_some();

		Some(true).xpect().as_some().to_be(true);
	}
}
