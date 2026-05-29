//! no_std future-joining helpers.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::Poll;

/// Polls a collection of fallible futures concurrently, resolving to their
/// outputs in iteration order once all succeed, or short-circuiting on the
/// first [`Err`].
///
/// A no_std drop-in for `futures::future::try_join_all`, backed only by
/// `alloc` + `core`.
pub async fn try_join_all<Fut, T, E>(
	futures: impl IntoIterator<Item = Fut>,
) -> Result<Vec<T>, E>
where
	Fut: Future<Output = Result<T, E>>,
{
	let mut futures: Vec<Option<Pin<Box<Fut>>>> =
		futures.into_iter().map(|fut| Some(Box::pin(fut))).collect();
	let mut results: Vec<Option<T>> =
		core::iter::repeat_with(|| None).take(futures.len()).collect();

	core::future::poll_fn(move |cx| {
		let mut all_done = true;
		for (idx, slot) in futures.iter_mut().enumerate() {
			let Some(fut) = slot else { continue };
			match fut.as_mut().poll(cx) {
				Poll::Ready(Ok(value)) => {
					results[idx] = Some(value);
					*slot = None;
				}
				Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
				Poll::Pending => all_done = false,
			}
		}
		if all_done {
			Poll::Ready(Ok(results.iter_mut().map(|r| r.take().unwrap()).collect()))
		} else {
			Poll::Pending
		}
	})
	.await
}
