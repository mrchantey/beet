//! copied from https://github.com/rust-lang/rust/blob/a25032cf444eeba7652ce5165a2be450430890ba/library/test/src/types.rs#L130
use test::Bencher;
// use test::CompletedTest;

pub enum Runnable {
	Test(RunnableTest),
	Bench(RunnableBench),
}

pub enum RunnableTest {
	Static(fn() -> Result<(), String>),
	Dynamic(Box<dyn FnOnce() -> Result<(), String> + Send>),
	StaticBenchAsTest(fn(&mut Bencher) -> Result<(), String>),
	DynamicBenchAsTest(Box<dyn Fn(&mut Bencher) -> Result<(), String> + Send>),
}

impl RunnableTest {
	// pub fn run(self) -> Result<(), String> {
	// 	match self {
	// 		RunnableTest::Static(f) => __rust_begin_short_backtrace(f),
	// 		RunnableTest::Dynamic(f) => __rust_begin_short_backtrace(f),
	// 		RunnableTest::StaticBenchAsTest(f) => crate::bench::run_once(|b| {
	// 			__rust_begin_short_backtrace(|| f(b))
	// 		}),
	// 		RunnableTest::DynamicBenchAsTest(f) => {
	// 			crate::bench::run_once(|b| {
	// 				__rust_begin_short_backtrace(|| f(b))
	// 			})
	// 		}
	// 	}
	// }

	pub fn is_dynamic(&self) -> bool {
		match self {
			RunnableTest::Static(_) => false,
			RunnableTest::StaticBenchAsTest(_) => false,
			RunnableTest::Dynamic(_) => true,
			RunnableTest::DynamicBenchAsTest(_) => true,
		}
	}
}

pub enum RunnableBench {
	Static(fn(&mut Bencher) -> Result<(), String>),
	Dynamic(Box<dyn Fn(&mut Bencher) -> Result<(), String> + Send>),
}

// impl RunnableBench {
// 	pub fn run(
// 		self,
// 		id: TestId,
// 		desc: &TestDesc,
// 		monitor_ch: &Sender<CompletedTest>,
// 		nocapture: bool,
// 	) {
// 		match self {
// 			RunnableBench::Static(f) => test::bench::benchmark(
// 				id,
// 				desc.clone(),
// 				monitor_ch.clone(),
// 				nocapture,
// 				f,
// 			),
// 			RunnableBench::Dynamic(f) => test::bench::benchmark(
// 				id,
// 				desc.clone(),
// 				monitor_ch.clone(),
// 				nocapture,
// 				f,
// 			),
// 		}
// 	}
// }
