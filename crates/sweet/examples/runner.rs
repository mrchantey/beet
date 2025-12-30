//! Example usage of the test runner
use sweet::prelude::*;



fn main() {
	sweet::test_runner2(&[
		&test_ext::new_auto_static(returns_ok),
		&test_ext::new_auto_static(returns_err).with_ignore(true),
		&test_ext::new_auto_static(panics).with_should_panic(),
		&test_ext::new_auto_static(returns_ok_async),
		&test_ext::new_auto_static(returns_err_async).with_ignore(true),
		&test_ext::new_auto_static(panics_async).with_should_panic(),
	]);
}


fn returns_ok() -> Result<(), String> { Ok(()) }
fn returns_err() -> Result<(), String> { Err("foo".to_string()) }
fn panics() -> Result<(), String> {
	panic!("whoops");
}


fn returns_ok_async() -> Result<(), String> {
	register_async_test(async { Ok(()) });
	Ok(())
}
fn returns_err_async() -> Result<(), String> {
	register_async_test(async { Err("foo".to_string()) });
	Ok(())
}
fn panics_async() -> Result<(), String> {
	register_async_test(async {
		panic!("whoops");
	});
	Ok(())
}
