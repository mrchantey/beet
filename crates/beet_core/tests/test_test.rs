use beet_core::prelude::*;

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

#[beet_core::test]
async fn returns_ok_async() { time_ext::sleep_millis(10).await; }

#[beet_core::test]
async fn beet_test_async() { time_ext::sleep_millis(10).await; }

#[beet_core::test]
fn beet_test_sync() {}

/// Panics inside an async test; the explicit `Ok(())` ensures the
/// return type is unambiguous on stable (no never-type inference).
#[test]
#[should_panic]
fn panics_async() {
	beet_core::testing::block_on_async_test(async {
		panic!();
		#[allow(unreachable_code)]
		Ok::<(), String>(())
	});
}

#[cfg(feature = "custom_test_framework")]
mod nightly_only {
	use beet_core::prelude::*;

	#[beet_core::test(timeout_ms = 10_000)]
	#[ignore = "very slow"]
	async fn timeout_respected() {
		// default timeout is 5 seconds
		time_ext::sleep_secs(6).await;
	}
}
