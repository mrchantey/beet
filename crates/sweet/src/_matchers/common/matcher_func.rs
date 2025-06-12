use crate::prelude::*;
use std::fmt::Debug;

impl<I, O, F> Matcher<&MockFunc<I, O, F>> {
	pub fn to_have_been_called(&self) {
		let received = self.value.called.lock().unwrap().len();
		self.assert_correct_with_received(
			received > 0,
			&"to have been called",
			&false,
		);
	}
	pub fn to_have_been_called_times(&self, times: usize) {
		let received = self.value.called.lock().unwrap().len();
		self.assert_correct_with_received(
			received == times,
			&format!("to have been called {times} times"),
			&format!("called {received} times"),
		);
	}
}
impl<I, O: Clone, F> Matcher<&MockFunc<I, O, F>> {
	pub fn nth_return(&self, time: usize) -> Matcher<O> {
		let vec = self.value.called.lock().unwrap();
		if let Some(received) = vec.get(time) {
			Matcher::new(received.clone())
		} else {
			self.assert_with_expected_received(
				&"to have been called",
				&"not called",
			);
		}
	}
}
impl<I, O: Debug + PartialEq, F> Matcher<&MockFunc<I, O, F>> {
	/// checks the first time it was called
	pub fn to_have_returned_with(&self, expected: O) {
		if let Some(received) = self.value.called.lock().unwrap().first() {
			self.assert_correct_with_received(
				received == &expected,
				&expected,
				received,
			);
		} else {
			self.assert_with_expected_received(
				&"to have been called",
				&"not called",
			);
		}
	}
	pub fn to_have_returned_nth_with(&self, time: usize, expected: &O) {
		if let Some(received) = self.value.called.lock().unwrap().get(time) {
			self.assert_correct_with_received(
				received == expected,
				expected,
				received,
			);
		} else {
			self.panic_with_expected_received(
				&"to have been called",
				&"not called",
			)
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	#[test]
	fn test_mock_trigger() {
		let func = mock_trigger();
		func.call(());
		func.call(());
		expect(&func).to_have_been_called();
		expect(&func).to_have_been_called_times(2);
		expect(&func.clone()).not().to_have_been_called_times(1);
	}
	#[test]
	fn test_mock_func() {
		let func = mock_func(|i| i * 2);
		func.call(0);
		func.call(2);
		expect(&func).to_have_been_called();
		expect(&func).to_have_returned_with(0);
		expect(&func).not().to_have_returned_with(4);
		expect(&func).nth_return(1).to_be(4);
		expect(&func).nth_return(0).to_be(0);
		expect(&func).nth_return(1).to_be(4);
	}
}
