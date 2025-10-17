pub use futures::future::try_join_all;
use futures_lite::future::YieldNow;
use std::pin::Pin;


pub fn block_on<F: Future>(fut: F) -> F::Output {
	futures::executor::block_on(fut)
}
pub fn yield_now() -> YieldNow { futures_lite::future::yield_now() }

/// A 'static + Send, making it suitable for use-cases like tokio::spawn
pub type SendBoxedFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;

/// A BoxedFuture which is `Send` on non-wasm32 targets
#[cfg(target_arch = "wasm32")]
pub type MaybeSendBoxedFuture<T> = Pin<Box<dyn 'static + Future<Output = T>>>;
/// A BoxedFuture which is `Send` on non-wasm32 targets
#[cfg(not(target_arch = "wasm32"))]
pub type MaybeSendBoxedFuture<T> =
	Pin<Box<dyn 'static + Send + Future<Output = T>>>;

/// Cross platform spawn_local function
#[cfg(target_arch = "wasm32")]
pub fn spawn_local<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut)
}
/// Cross platform spawn_local function
// TODO deprecate for async-executor or bevy tasks?
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
pub fn spawn_local<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
where
	F: Future + 'static,
	F::Output: 'static,
{
	tokio::task::spawn_local(fut)
}


#[cfg(all(not(target_arch = "wasm32"), not(feature = "tokio")))]
pub fn spawn_local<F>(_: F)
where
	F: Future<Output = ()> + 'static,
{
	unimplemented!("please enable tokio feature")
}
