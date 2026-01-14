#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use sweet::prelude::*;

#[test]
fn returns_ok() -> Result<(), String> { Ok(()) }


#[test]
#[ignore]
fn ignored() {
	panic!();
}

#[test]
#[should_panic]
fn should_panic() {
	panic!();
}

#[test]
fn returns_ok_async() {
	register_sweet_test(TestCaseParams::new(), async {
		time_ext::sleep_millis(10).await;
		Ok(())
	});
}
#[test]
#[should_panic]
fn panics_async() {
	register_sweet_test(TestCaseParams::new(), async {
		panic!();
	});
}
#[sweet::test]
fn sweet_test_sync() {}

#[sweet::test]
async fn sweet_test_async() { time_ext::sleep_millis(10).await; }
