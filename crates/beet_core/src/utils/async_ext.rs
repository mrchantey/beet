pub use futures::future::try_join_all;
use std::pin::Pin;

/// A 'static + Send, making it suitable for use-cases like tokio::spawn
pub type SendBoxedFuture<T> = Pin<Box<dyn 'static + Send + Future<Output = T>>>;

/// Cross platform spawn_local function
#[cfg(target_arch = "wasm32")]
pub fn spawn_local<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut)
	// tokio::task::spawn_local(fut).await.expect("Task panicked")
}
/// Cross platform spawn_local function
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
