use crate::prelude::*;
use anyhow::Result;
use flume::Sender;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::cell::Cell;
use std::sync::Arc;
use test::TestDesc;
use thread_local::ThreadLocal;

pub fn rayon_with_num_threads(
	test_threads: Option<usize>,
) -> Result<rayon::ThreadPool> {
	let mut local_pool = rayon::ThreadPoolBuilder::new();
	if let Some(test_threads) = test_threads {
		local_pool = local_pool.num_threads(test_threads);
	}
	let pool = local_pool.build()?;
	Ok(pool)
}

/// Best runner for mostly native sync tests.
/// A seperate [futures::executor] will be lazily spawned for each async test
/// as it is found, and block on its completion.
pub struct TestRunnerRayon;


impl TestRunner for TestRunnerRayon {
	fn run(
		config: &TestRunnerConfig,
		future_tx: Sender<TestDescAndFuture>,
		result_tx: Sender<TestDescAndResult>,
		tests: Vec<test::TestDescAndFn>,
	) -> Result<()> {
		// let tls = Arc::new(ThreadLocal::new());


		let local_pool = rayon_with_num_threads(config.test_threads)?;

		let tls_desc = Arc::new(ThreadLocal::<Cell<Option<TestDesc>>>::new());
		let default_hook = std::panic::take_hook();

		let tls_desc2 = tls_desc.clone();
		let result_tx2 = result_tx.clone();
		std::panic::set_hook(Box::new(move |info| {
			if let Some(desc) = tls_desc2.get() {
				if let Some(desc) = desc.take() {
					let result = TestResult::from_panic(info, &desc);
					if let Err(err) =
						result_tx2.send(TestDescAndResult::new(desc, result))
					{
						eprintln!("failed to register panic: {}", err);
					}
				} else {
					eprintln!("malformed thread local test description");
				}
			} else {
				default_hook(info);
			}
		}));

		let _results = local_pool
			.install(|| {
				tests.into_par_iter().map_with(
					tls_desc.clone(),
					|desc_cell, test| {
						let tls_desc_cell =
							desc_cell.get_or(|| Default::default());
						tls_desc_cell.set(Some(test.desc.clone()));

						let func = TestDescAndFnExt::func(&test);
						let result =
							SweetTestCollector::with_scope(&test.desc, || {
								std::panic::catch_unwind(func)
							});
						match result {
							Ok(Ok(result)) => {
								result_tx
									.send(TestDescAndResult::new(
										test.desc.clone(),
										TestResult::from_test_result(
											result, &test.desc,
										),
									))
									.expect("channel was dropped");
								// None
							}
							Ok(Err(_payload)) => {
								// panic result was sent in the hook
							}
							Err(fut) => {
								future_tx.send(fut).unwrap();
							}
						};
						let cell = desc_cell.get_or(|| Default::default());
						cell.set(None);
						// return output;
					},
				)
			})
			.collect::<Vec<_>>();


		let _hook = std::panic::take_hook();

		Ok(())
	}
}
