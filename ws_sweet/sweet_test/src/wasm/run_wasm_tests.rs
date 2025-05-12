use crate::prelude::*;
use flume::Sender;
use test::TestDescAndFn;


/// Run sync tests, and return async tests
pub fn run_wasm_tests_sync(
	tests: Vec<TestDescAndFn>,
	result_tx: &Sender<TestDescAndResult>,
) -> Vec<TestDescAndFuture> {
	tests
		.into_iter()
		.filter_map(|test| {
			let mut func = TestDescAndFnExt::func(&test);

			let result = SweetTestCollector::with_scope(&test.desc, || {
				PanicStore::with_scope(&test.desc, || {
					js_runtime::panic_to_error(&mut func)
				})
			});

			match result {
				Ok(panic_out) => {
					panic_out.send(result_tx, &test.desc);
					None
				}
				Err(val) => Some(val),
			}
		})
		.collect()
}
