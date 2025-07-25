use super::*;
use std::ops::ControlFlow;

impl<B, C> Matcher<ControlFlow<B, C>> {
	pub fn to_continue(&self) {
		match &self.value {
			ControlFlow::Continue(_) => {}
			ControlFlow::Break(_) => {
				self.assert_correct_with_received(
					false,
					&"ControlFlow::Continue",
					&"ControlFlow::Break",
				);
			}
		}
	}
	pub fn to_break(&self) {
		match &self.value {
			ControlFlow::Break(_) => {}
			ControlFlow::Continue(_) => {
				self.assert_correct_with_received(
					false,
					&"ControlFlow::Break",
					&"ControlFlow::Continue",
				);
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn bool() {
		// expect(true).to_be_false();
		expect(true).to_be_true();
		expect(false).not().to_be_true();

		expect(false).to_be_false();
		expect(true).not().to_be_false();
	}
}
