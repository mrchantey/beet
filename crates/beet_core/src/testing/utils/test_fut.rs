use std::future::Future;

/// A trait representing an async test future that returns a Result<(), String>
pub trait AsyncTest: 'static + Future<Output = Result<(), String>> {}
impl<F> AsyncTest for F where F: 'static + Future<Output = Result<(), String>> {}




/// Converts a type into an async test future.
pub trait IntoFut<M> {
	/// Converts this type into an async test future.
	fn into_fut(self) -> impl AsyncTest;
}
/// Marker for futures that return `Result<(), String>`.
pub struct ReturnsResult;
/// Marker for futures that return `()`.
pub struct ReturnsUnit;
/// Marker for futures that return `!` (never type).
pub struct ReturnsNever;

impl<T> IntoFut<ReturnsResult> for T
where
	T: AsyncTest,
{
	fn into_fut(self) -> impl AsyncTest { self }
}
impl<T> IntoFut<ReturnsUnit> for T
where
	T: 'static + Future<Output = ()>,
{
	fn into_fut(self) -> impl AsyncTest {
		async move {
			self.await;
			Ok(())
		}
	}
}
impl<T> IntoFut<ReturnsNever> for T
where
	T: 'static + Future<Output = !>,
{
	fn into_fut(self) -> impl AsyncTest {
		async move {
			self.await;
		}
	}
}

/// Just block on async tests, useful as fallback
/// for when runner feature is disabled.
#[track_caller]
pub fn block_on_async_test<M>(fut: impl IntoFut<M>) {
	futures_lite::future::block_on(fut.into_fut()).unwrap();
}
