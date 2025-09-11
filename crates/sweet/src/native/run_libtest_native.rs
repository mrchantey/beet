// use super::run_test::run_test;
use crate::prelude::*;
use std::sync::Arc;
extern crate test;
use anyhow::Result;
/// maybe we can allow test_main_with_filenames() as a feature

pub fn run_libtest_native(tests: &[&test::TestDescAndFn]) -> Result<()> {
	let (future_tx, _future_rx) = flume::unbounded::<TestDescAndFuture>();
	let (result_tx, result_rx) = flume::unbounded::<TestDescAndResult>();
	let config = Arc::new(TestRunnerConfig::from_env_args());

	let mut logger = RunnerLogger::start(config.clone(), &tests);

	let recv_result_handle = std::thread::spawn(move || {
		tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("Failed building the Runtime")
			.block_on(async move {
				while let Ok(result) = result_rx.recv_async().await {
					logger.on_result(result)?;
				}
				logger.end();
				Ok::<(), anyhow::Error>(())
			})
	});

	TestRunnerRayon::collect_and_run(&config, future_tx, result_tx, tests)
		.unwrap();
	recv_result_handle.join().unwrap()?;

	Ok(())
}
