#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn test_mock_func() {
		let func = beet_utils::arena::mock_func::<i32, i32, _>(|i: i32| i * 2);
		func.call(0);
		func.call(2);
		func.called.len().xpect_eq(2);
		func.called.get()[0].xpect_eq(0);
		func.called.get()[1].xpect_eq(4);
	}
}
