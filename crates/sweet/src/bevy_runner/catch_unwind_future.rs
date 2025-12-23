// pub enum CatchUnwindFutureOut{
// 	Ok,
// 	Panicked,
// }

// use std::any::Any;
// use std::panic::Location;
// use std::panic::PanicInfo;
// #[cfg(not(target_arch = "wasm32"))]
// use std::pin::Pin;
// #[cfg(not(target_arch = "wasm32"))]
// use std::task::Context;
// #[cfg(not(target_arch = "wasm32"))]
// use std::task::Poll;

// struct OwnedPanicInfo {
// 	payload: Box<dyn Any + Send>,
// 	location: Location<'static>,
// 	can_unwind: bool,
// 	force_no_backtrace: bool,
// }
// pub struct CatchUnwindFuture<Func> {
// 	func: Func,
// }

// impl<Func> CatchUnwindFuture<Func> {
// 	pub fn new(func: Func) -> Self { Self { func } }
// }

// pub enum CatchUnwindResult {
// 	Ok,
// 	Err(String),
// 	Panicked(OwnedPanicInfo),
// }


// #[cfg(not(target_arch = "wasm32"))]
// impl<F: Future<Output = Result<(), String>>> Future for CatchUnwindFuture<F> {
// 	type Output = Result<Result<(), String>, OwnedPanicInfo>;

// 	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
// 		// SAFETY: we never move inner after pinning
// 		let this = unsafe { self.get_unchecked_mut() };
// 		let inner = unsafe { Pin::new_unchecked(&mut this.func) };

// 		// Wrap this single poll in catch_unwind
// 		let result =
// 			std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
// 				inner.poll(cx)
// 			}));

// 		match result {
// 			Ok(Poll::Ready(Ok(()))) => Poll::Ready(Ok(())),
// 			Ok(Poll::Ready(Err(msg))) => {
// 				Poll::Ready(TestOutcome::Err { message: msg })
// 			}
// 			Ok(Poll::Pending) => Poll::Pending,
// 			Err(payload) => Poll::Ready(TestOutcome::Panic {
// 				payload: panic_payload_to_string(payload),
// 			}),
// 		}
// 	}
// }
