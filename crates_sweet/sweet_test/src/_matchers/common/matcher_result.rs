use super::*;
use std::fmt::Debug;
use std::fmt::Display;

impl<T: Debug, E: Debug> Matcher<Result<T, E>> {
	pub fn to_be_ok_val(&self, val: impl PartialEq<T> + Debug) {
		if let Ok(v) = &self.value {
			let result = val == *v;
			self.assert_correct(result, &format!("{:?}", val));
		} else {
			self.assert_correct_with_received(false, &"Ok", &"Error");
		}
	}
	pub fn to_be_ok(&self) {
		let result = self.value.is_ok();
		self.assert_correct(result, &"Ok");
	}
	pub fn to_be_err(&self) {
		let result = self.value.is_err();
		self.assert_correct(result, &"Error");
	}
}
// TODO T shouldt need to be debug
impl<T: Debug, E: Debug + Display> Matcher<Result<T, E>> {
	pub fn to_be_err_str(&self, value: &str) {
		if let Err(err) = &self.value {
			let result = err.to_string() == value;
			self.assert_correct(result, &value);
		} else {
			self.assert_correct_with_received(false, &"Error", &"Ok");
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::anyhow;

	#[test]
	fn result() {
		let ok = || -> anyhow::Result<()> { Ok(()) };
		expect(ok()).to_be_ok();
		expect(ok()).not().to_be_err();

		let err = || -> anyhow::Result<()> { Err(anyhow!("foo")) };

		expect(err()).to_be_err();
		expect(err()).not().to_be_ok();

		expect(err()).to_be_err_str("foo");
		expect(err()).not().to_be_err_str("foobar");
	}
}
