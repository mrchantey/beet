use crate::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use test::TestDesc;

pub trait SweetTestFuture:
	'static + Future<Output = Result<(), String>>
{
}
impl<T> SweetTestFuture for T where
	T: 'static + Future<Output = Result<(), String>>
{
}


pub type SweetFutFunc =
	Box<dyn Send + Sync + Fn() -> Pin<Box<dyn SweetTestFuture>>>;

type FutCell = Arc<Mutex<Option<SweetFutFunc>>>;

thread_local! {
	static FUTURE: FutCell = Arc::new(Mutex::new(None));
}

pub struct SweetTestCollector;

impl SweetTestCollector {
	/// # Panics
	/// If called outside of [`Self::set`]
	pub fn register<F: SweetTestFuture>(fut: fn() -> F) {
		FUTURE.with(|current_fut| {
			*current_fut.lock().unwrap() =
				Some(Box::new(move || Box::pin(fut())));
		});
	}

	/// This function uses the Error type to represent
	/// that a future has been registered
	pub fn with_scope<F, R>(
		desc: &TestDesc,
		func: F,
	) -> Result<R, TestDescAndFuture>
	where
		F: FnOnce() -> R,
	{
		// let val = Arc::new(Mutex::new(None));
		FUTURE.with(|val| {
			let out = func();
			if let Some(fut) = val.lock().unwrap().take() {
				Err(TestDescAndFuture::new(desc.clone(), fut))
			} else {
				Ok(out)
			}
		})
	}
}
