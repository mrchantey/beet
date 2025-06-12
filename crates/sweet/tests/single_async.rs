//! example usage of async tests
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

// ensure we are not nesting runtimes or get this error:
// Cannot start a runtime from within a runtime. This happens because a function (like `block_on`) attempted to block the current thread while the thread is being used to drive asynchronous tasks.
#[tokio::test]
#[cfg(not(target_arch = "wasm32"))]
async fn dummy1() {
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
