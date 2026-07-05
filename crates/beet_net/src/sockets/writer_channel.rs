//! A minimal no_std + alloc, waker-based unbounded mpsc for the
//! [`Socket::effect`](super::Socket) writer-feed: the sync `MessageSend` observer
//! pushes, the async writer task drains in FIFO order.
//!
//! `futures-channel`'s `mpsc` and `async-channel` are both std-only, so the
//! agnostic socket core carries its own tiny channel here. It stays engine- and
//! target-agnostic (esp specifics use `embassy_sync` in the downstream transport,
//! never this). The [`Receiver`] ends (`recv` returns `None`) once the channel is
//! empty and every [`Sender`] has dropped, matching `async-channel`'s close.

use alloc::collections::VecDeque;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::future::poll_fn;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::task::Context;
use core::task::Poll;
use core::task::Waker;

/// Create a connected [`Sender`] / [`Receiver`] pair.
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
	let shared = Arc::new(Shared {
		queue: Mutex::new(VecDeque::new()),
		waker: Mutex::new(None),
		senders: AtomicUsize::new(1),
	});
	(
		Sender {
			shared: shared.clone(),
		},
		Receiver { shared },
	)
}

/// Sending half. Cloneable (multi-producer); the [`Receiver`] ends once the last
/// clone drops.
pub struct Sender<T> {
	shared: Arc<Shared<T>>,
}

/// Receiving half, an async FIFO drain.
pub struct Receiver<T> {
	shared: Arc<Shared<T>>,
}

struct Shared<T> {
	queue: Mutex<VecDeque<T>>,
	waker: Mutex<Option<Waker>>,
	senders: AtomicUsize,
}

impl<T> Sender<T> {
	/// Enqueue a value and wake the receiver.
	pub fn send(&self, value: T) {
		self.shared.queue.lock().unwrap().push_back(value);
		self.wake_receiver();
	}

	fn wake_receiver(&self) {
		if let Some(waker) = self.shared.waker.lock().unwrap().take() {
			waker.wake();
		}
	}
}

impl<T> Clone for Sender<T> {
	fn clone(&self) -> Self {
		self.shared.senders.fetch_add(1, Ordering::Relaxed);
		Self {
			shared: self.shared.clone(),
		}
	}
}

impl<T> Drop for Sender<T> {
	fn drop(&mut self) {
		// last sender gone: wake the receiver so it observes the close.
		if self.shared.senders.fetch_sub(1, Ordering::AcqRel) == 1 {
			self.wake_receiver();
		}
	}
}

impl<T> Receiver<T> {
	/// Await the next value, or `None` once the channel is empty and every
	/// [`Sender`] has dropped.
	pub async fn recv(&self) -> Option<T> {
		poll_fn(|cx| self.poll_recv(cx)).await
	}

	/// Poll for the next value, `Ready(None)` once the channel is empty and
	/// every [`Sender`] has dropped — the seam for a `Stream` wrapper (eg a
	/// transport's [`Socket`](super::Socket) reader).
	pub fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<Option<T>> {
		if let Some(value) = self.shared.queue.lock().unwrap().pop_front() {
			return Poll::Ready(Some(value));
		}
		if self.is_closed() {
			return Poll::Ready(None);
		}
		// register for wakeup, then re-check to close the lost-notification race
		// (a send + last-sender-drop between the pop above and here).
		*self.shared.waker.lock().unwrap() = Some(cx.waker().clone());
		if let Some(value) = self.shared.queue.lock().unwrap().pop_front() {
			Poll::Ready(Some(value))
		} else if self.is_closed() {
			Poll::Ready(None)
		} else {
			Poll::Pending
		}
	}

	fn is_closed(&self) -> bool {
		self.shared.senders.load(Ordering::Acquire) == 0
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn delivers_in_fifo_order() {
		let (send, recv) = unbounded::<u32>();
		send.send(1);
		send.send(2);
		recv.recv().await.unwrap().xpect_eq(1);
		recv.recv().await.unwrap().xpect_eq(2);
	}

	#[beet_core::test]
	async fn closes_when_senders_drop() {
		let (send, recv) = unbounded::<u32>();
		send.send(7);
		drop(send);
		// buffered items drain first, then the stream ends.
		recv.recv().await.unwrap().xpect_eq(7);
		recv.recv().await.is_none().xpect_true();
	}
}
