//! Exercises the test runner itself. This is the one target that also
//! validates the nightly libtest / `custom_test_frameworks` path; with the
//! feature off it runs via the stable `inventory` runner.
#![cfg_attr(feature = "custom_test_frameworks", allow(unused_features))]
#![cfg_attr(
	feature = "custom_test_frameworks",
	feature(test, custom_test_frameworks)
)]
#![cfg_attr(
	feature = "custom_test_frameworks",
	test_runner(beet_core::libtest_runner)
)]
use beet_core::prelude::*;

// Default (stable) path: this target keeps libtest's default harness, so a
// single `#[test]` drives the inventory-registered cases. On the nightly
// `custom_test_frameworks` path the `#![test_runner]` above takes over.
#[cfg(not(feature = "custom_test_frameworks"))]
#[test]
fn __beet_inventory() { beet_core::testing::test_main(); }

use beet_core::testing;

#[beet_core::test]
fn returns_ok() -> Result<(), String> { Ok(()) }

#[beet_core::test]
#[ignore]
fn ignored() {
	panic!();
}

#[beet_core::test]
#[should_panic]
fn should_panic() {
	panic!();
}

#[beet_core::test]
fn returns_ok_async() {
	register_test(TestCaseParams::new(), async {
		time_ext::sleep_millis(10).await;
		Ok::<(), String>(())
	});
}

#[beet_core::test]
#[should_panic]
fn panics_async() {
	register_test(TestCaseParams::new(), async {
		panic!();
		#[allow(unreachable_code)]
		Ok::<(), String>(())
	});
}

#[beet_core::test]
fn beet_test_sync() {}

#[beet_core::test]
async fn beet_test_async() { time_ext::sleep_millis(10).await; }

#[beet_core::test(timeout_ms = 10_000)]
#[ignore = "very slow"]
async fn timeout_respected() {
	// default timeout is 5 seconds
	time_ext::sleep_secs(6).await;
}
