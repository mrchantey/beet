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
		// true.xpect().to_be_false();
		true.xpect().to_be_true();
		false.xpect().not().to_be_true();

		false.xpect().to_be_false();
		true.xpect().not().to_be_false();
	}
}
