use crate::prelude::*;
use flume::Receiver;
use flume::Sender;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
	static STORE: Rc<RefCell<Option<PartialRunnerState>>> = Default::default();
}

/// wasm needs to exit and re-enter the test runner
/// in order to execute async tests, so save the partial state.
/// Even though [wasm bindgen supports main](https://rustwasm.github.io/wasm-bindgen/reference/attributes/on-rust-exports/main.html)
/// custom_test_frameworks do not.
pub struct PartialRunnerState {
	pub logger: RunnerLogger,
	pub futures: Vec<TestDescAndFuture>,
	pub result_tx: Sender<TestDescAndResult>,
	pub result_rx: Receiver<TestDescAndResult>,
}


impl PartialRunnerState {
	pub fn set(self) {
		STORE.with(|store| {
			*store.borrow_mut() = Some(self);
		})
	}
	pub fn take() -> Option<Self> {
		STORE.with(|store| std::mem::take(&mut *store.borrow_mut()))
	}
}
