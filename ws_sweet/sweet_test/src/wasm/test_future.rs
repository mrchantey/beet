//! donation from the good folks at wasm-bindgen 🙏
//! https://github.com/rustwasm/wasm-bindgen/blob/24f20ae9bc480c5ad0778fdb1481eb23461f0d82/crates/test/src/rt/mod.rs#L764
use crate::prelude::*;
use flume::Sender;
use std::future::Future;
use std::pin::Pin;
use std::task;
use std::task::Poll;
use test::TestDesc;
use wasm_bindgen::prelude::*;

pub struct TestFuture<F> {
	desc: TestDesc,
	result_tx: Sender<TestDescAndResult>,
	test: F,
}

impl<F> TestFuture<F> {
	pub fn new(
		desc: TestDesc,
		result_tx: Sender<TestDescAndResult>,
		test: F,
	) -> Self {
		Self {
			desc,
			result_tx,
			test,
		}
	}
}

impl<F: Future<Output = Result<(), String>>> Future for TestFuture<F> {
	type Output = ();

	fn poll(
		self: Pin<&mut Self>,
		cx: &mut task::Context,
	) -> Poll<Self::Output> {
		// let output = self.output.clone();
		// Use `new_unchecked` here to project our own pin, and we never
		// move `test` so this should be safe

		let self_mut = unsafe { self.get_unchecked_mut() };
		let test = unsafe { Pin::new_unchecked(&mut self_mut.test) };
		let desc = &self_mut.desc;
		let result_tx = &self_mut.result_tx;
		let mut future_poll_output = None;

		// this should only be used if poll::ready!
		let panic_output = PanicStore::with_scope(desc, || {
			let mut test = Some(test);
			js_runtime::panic_to_error(&mut || {
				let test = test.take().unwrap_throw();
				let out = test.poll(cx);
				future_poll_output = Some(match &out {
					Poll::Ready(_) => Poll::Ready(()),
					Poll::Pending => Poll::Pending,
				});
				match out {
					Poll::Ready(Err(err)) => Err(err),
					_ => Ok(()),
				}
			})
		});

		// panicked futures will never be ready
		if panic_output.panicked() {
			panic_output.send(result_tx, desc);
			return Poll::Ready(());
		}

		match future_poll_output {
			Some(Poll::Pending) => Poll::Pending,
			Some(Poll::Ready(())) => {
				panic_output.send(result_tx, desc);
				Poll::Ready(())
			}
			None => Poll::Pending,
		}
	}
}
