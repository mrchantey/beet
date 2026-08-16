//! A correct [`Stream`] over an [`async_channel::Receiver`], the delivery
//! half of every browser-callback channel here (events, frames, ticks).
//!
//! The trap this exists to avoid: polling an ad-hoc `recv()` future inside
//! `poll_next` and dropping it on `Pending` deregisters its waker with it, so
//! the task is never woken for the item that does arrive. This wrapper holds
//! its recv future across polls instead.

use async_channel::Receiver;
use async_channel::RecvError;
use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use futures_lite::Stream;

type RecvFuture<T> = Pin<Box<dyn Future<Output = Result<T, RecvError>>>>;

/// Stream adapter over a callback channel's receiver.
pub(crate) struct RecvStream<T> {
	receiver: Receiver<T>,
	/// The in-flight recv, held across polls so its waker stays registered.
	pending: Option<RecvFuture<T>>,
}

impl<T> Unpin for RecvStream<T> {}

impl<T> RecvStream<T> {
	pub fn new(receiver: Receiver<T>) -> Self {
		Self {
			receiver,
			pending: None,
		}
	}

	/// Await the next item directly; the future is held by the caller, so
	/// waker registration is sound without the stored pending slot.
	pub async fn recv(&self) -> Option<T> { self.receiver.recv().await.ok() }
}

impl<T: 'static> Stream for RecvStream<T> {
	type Item = T;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<T>> {
		let this = self.get_mut();
		let pending = this.pending.get_or_insert_with(|| {
			let receiver = this.receiver.clone();
			Box::pin(async move { receiver.recv().await })
		});
		match pending.as_mut().poll(cx) {
			Poll::Ready(result) => {
				this.pending = None;
				Poll::Ready(result.ok())
			}
			Poll::Pending => Poll::Pending,
		}
	}
}
