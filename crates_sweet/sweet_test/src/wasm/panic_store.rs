use crate::prelude::*;
use core::fmt;
use flume::Sender;
use std::cell::RefCell;
use std::panic::PanicHookInfo;
use std::rc::Rc;
use test::TestDesc;
use wasm_bindgen::JsValue;


/// A completed test that maybe panicked
pub enum PanicStoreOut<T> {
	Panicked(TestDescAndResult),
	/// maybe returned error
	NoPanic(T),
}

impl<T> PanicStoreOut<T> {
	pub fn panicked(&self) -> bool {
		match self {
			PanicStoreOut::Panicked(_) => true,
			PanicStoreOut::NoPanic(_) => false,
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for PanicStoreOut<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			PanicStoreOut::Panicked(result) => {
				write!(f, "Panicked({:?})", result)
			}
			PanicStoreOut::NoPanic(result) => {
				write!(f, "NoPanic({:?})", result)
			}
		}
	}
}

impl<T> PanicStoreOut<Result<T, JsValue>> {
	pub fn send(self, result_tx: &Sender<TestDescAndResult>, desc: &TestDesc) {
		match self {
			PanicStoreOut::Panicked(result) => {
				result_tx.send(result).expect("channel was dropped");
			}
			PanicStoreOut::NoPanic(Err(err)) => {
				let err = if err.is_string() {
					err.as_string().unwrap()
				} else {
					format!("{:?}", err)
				};


				let test_result = TestResult::from_test_result(Err(err), desc);
				result_tx
					.send(TestDescAndResult::new(desc.clone(), test_result))
					.expect("channel was dropped");
			}
			PanicStoreOut::NoPanic(Ok(_)) => {
				let test_result = TestResult::from_test_result(Ok(()), desc);
				result_tx
					.send(TestDescAndResult::new(desc.clone(), test_result))
					.expect("channel was dropped");
			}
		}
	}
}


crate::scoped_thread_local! {
	static CURRENT_LISTENER: Rc<RefCell<(TestDesc,Option<TestResult>)>>
}

/// when a test panics, store it globally
/// and retrieve immediately after
pub struct PanicStore;

impl PanicStore {
	// it seems in wasm we can only set_hook once, otherwise
	// the setting of a hook itsself will panic
	/// This will be called from inside thie function
	/// at some point duing a Scoped Set
	pub fn panic_hook(info: &PanicHookInfo) {
		if !CURRENT_LISTENER.is_set() {
			// nobody is listening, must be a real one
			let payload = payload_to_string(info.payload());
			sweet_utils::log!("Sweet Runner Panic:\nThis is an internal sweet panic, please file an issue\n{}\nend payload", payload);
			return;
		} else {
			CURRENT_LISTENER.with(|last_panic| {
				let result =
					TestResult::from_panic(info, &last_panic.borrow().0);
				last_panic.borrow_mut().1 = Some(result);
			});
		}
	}

	// pub fn get() -> String {
	// 	CURRENT_LISTENER.with(|last_panic| last_panic.borrow().clone())
	// }



	/// if the function panics, and it should not have
	/// this will return None and emit a result.
	/// Otherwise deal with the function
	pub fn with_scope<F, R>(desc: &TestDesc, func: F) -> PanicStoreOut<R>
	where
		F: FnOnce() -> R,
	{
		let output = Rc::new(RefCell::new((desc.clone(), None)));
		CURRENT_LISTENER.set(&output, || {
			let test_out = func();
			match (output.borrow_mut().1.take(), test_out) {
				(Some(panic_result), _) => PanicStoreOut::Panicked(
					TestDescAndResult::new(desc.clone(), panic_result),
				),
				(None, result) => PanicStoreOut::NoPanic(result),
			}
		})
	}
}
