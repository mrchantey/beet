#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_core::testing;

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
	register_test(TestCaseParams::new(), async {
		time_ext::sleep_millis(10).await;
		Ok(())
	});
}
#[test]
#[should_panic]
fn panics_async() {
	register_test(TestCaseParams::new(), async {
		panic!();
	});
}
#[beet_core::test]
fn beet_test_sync() {}

#[beet_core::test]
async fn beet_test_async() { time_ext::sleep_millis(10).await; }

#[beet_core::test(timeout_ms = 10_000)]
#[ignore="very slow"]
// #[beet_core::test]
async fn timeout_respected() {
	// default timeout is 5 seconds
	time_ext::sleep_secs(6).await;
}
