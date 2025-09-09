use crate::prelude::*;
use crate::sweet_ref_impls;
use std::fmt::Debug;


sweet_ref_impls!(Vec; T);

#[extend::ext(name=VecMatcherExt)]
pub impl<T: Debug, U: Debug + SweetRef<Vec<T>>> Matcher<U> {
	fn to_be_empty(&self) -> &Self {
		self.assert_correct(self.value.sweet_ref().is_empty(), &"to be empty");
		self
	}
	fn to_have_length(&self, length: usize) -> &Self {
		self.assert_correct_with_received(
			self.value.sweet_ref().len() == length,
			&format!("to have length {}", length),
			&self.value.sweet_ref().len(),
		);
		self
	}
	fn any(&self, func: impl FnMut(&T) -> bool) -> &Self {
		self.assert_correct(
			self.value.sweet_ref().iter().any(func),
			&"any to match predicate",
		);
		self
	}
}


#[extend::ext(name=VecMatcherPartialEqExt)]
pub impl<T: Debug + PartialEq, U: Debug + SweetRef<Vec<T>>> Matcher<U> {
	fn to_contain_element(&self, other: &T) -> &Self {
		self.assert_correct(
			self.value.sweet_ref().contains(other),
			&format!("to contain {:?}", other),
		);
		self
	}
}


#[cfg(test)]
mod test {
	use beet_utils::prelude::*;

	use crate::prelude::*;

	#[test]
	fn vec() {
		vec![1, 2, 3].xpect().to_contain_element(&2);
		vec![1, 2, 3].xref().xpect().not().to_contain_element(&4);
		vec![1, 2, 3].xpect().any(|val| val == &2);
		(&vec![1, 2, 3]).xpect().not().any(|val| val == &4);
	}
}
