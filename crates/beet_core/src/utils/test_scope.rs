//! The test a thread is running code for, so a fault that escapes the test's
//! own catch scope is still attributed to it.
//!
//! The runner enters a test's scope around its call and each poll of its
//! future, and every task [`AsyncSpawner`] spawns carries the scope it was
//! spawned in, so a panic on a task pool thread or an error raised through
//! [`World::handle_command_error`](crate::prelude::WorldExt::handle_command_error)
//! names the test that caused it. [`PanicContext`] records each escape
//! against [`TestScope::current`].
use crate::prelude::*;
use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::task::Context;
use core::task::Poll;
use std::cell::Cell;

std::thread_local! {
	/// The scope of the test whose code this thread is running.
	static CURRENT: Cell<Option<TestScope>> = const { Cell::new(None) };
}

/// One test run's identity, unique across every runner in the process, a
/// runner nested inside a test included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct TestScope(u64);

impl TestScope {
	/// A scope no other test run shares.
	pub fn new() -> Self {
		static NEXT: AtomicU64 = AtomicU64::new(0);
		Self(NEXT.fetch_add(1, Ordering::Relaxed))
	}

	/// The scope this thread is running code for, if any.
	pub fn current() -> Option<Self> { CURRENT.with(Cell::get) }

	/// Run `func` in `scope`, restoring the previous scope after, even on
	/// unwind.
	pub fn enter<T>(scope: Option<Self>, func: impl FnOnce() -> T) -> T {
		let _restore = Restore(CURRENT.with(|cell| cell.replace(scope)));
		func()
	}

	/// `fut` with every poll run in this scope.
	pub fn scope<F: Future>(self, fut: F) -> Scoped<F> {
		Scoped {
			scope: Some(self),
			fut,
		}
	}

	/// Run `func`, marking every escape while it runs expected, for a test
	/// that escapes on purpose, ie a panicking thread it spawns. The mark is
	/// process wide, so an unrelated escape in the same window is excused too.
	pub fn expect_escapes<T>(func: impl FnOnce() -> T) -> T {
		let _close = CloseExpecting::open();
		func()
	}

	/// Whether a [`Self::expect_escapes`] scope is open.
	pub(crate) fn expecting() -> bool { EXPECTING.load(Ordering::SeqCst) > 0 }

	/// `fut` with every poll run in the scope current now, ie the scope a
	/// task is spawned in follows it to whatever thread polls it.
	pub fn propagate<F: Future>(fut: F) -> Scoped<F> {
		Scoped {
			scope: Self::current(),
			fut,
		}
	}
}

/// How many [`TestScope::expect_escapes`] scopes are open.
static EXPECTING: AtomicUsize = AtomicUsize::new(0);

/// An open [`TestScope::expect_escapes`] scope, closed on drop, even on
/// unwind.
struct CloseExpecting;

impl CloseExpecting {
	fn open() -> Self {
		EXPECTING.fetch_add(1, Ordering::SeqCst);
		Self
	}
}

impl Drop for CloseExpecting {
	fn drop(&mut self) { EXPECTING.fetch_sub(1, Ordering::SeqCst); }
}

/// Restores the scope a [`TestScope::enter`] replaced.
struct Restore(Option<TestScope>);

impl Drop for Restore {
	fn drop(&mut self) { CURRENT.with(|cell| cell.set(self.0)); }
}

/// A future polled in a [`TestScope`], see [`TestScope::scope`].
pub struct Scoped<F> {
	scope: Option<TestScope>,
	fut: F,
}

impl<F: Future> Future for Scoped<F> {
	type Output = F::Output;
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
		let scope = self.scope;
		// SAFETY: structural pinning of `fut`, which is never moved out of
		// `self`, and `Scoped` implements neither `Drop` nor `Unpin` by hand.
		let fut = unsafe { self.map_unchecked_mut(|this| &mut this.fut) };
		TestScope::enter(scope, || fut.poll(cx))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn enter_nests_and_restores() {
		let outer = TestScope::new();
		let inner = TestScope::new();
		TestScope::enter(Some(outer), || {
			TestScope::current().xpect_eq(Some(outer));
			TestScope::enter(Some(inner), || {
				TestScope::current().xpect_eq(Some(inner));
			});
			TestScope::current().xpect_eq(Some(outer));
		});
	}

	/// A task spawned in a scope is polled in it on whatever thread runs it,
	/// natively: wasm has the one thread.
	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn spawned_tasks_carry_the_scope() {
		let scope = TestScope::new();
		let task = TestScope::enter(Some(scope), || {
			TestScope::propagate(async { TestScope::current() })
		});
		std::thread::spawn(move || futures_lite::future::block_on(task))
			.join()
			.unwrap()
			.xpect_eq(Some(scope));
	}
}
