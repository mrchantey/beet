use crate::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

/// Ensures at most one async operation runs at a time, with one pending retry.
///
/// If [`start`] is called while an operation is in-flight, the trigger marks
/// itself dirty and returns `false`. When the in-flight operation calls
/// [`finish`], it returns `true` if dirty, prompting an immediate retry.
///
/// This prevents unbounded write queues while guaranteeing the latest state
/// is always flushed.
///
/// # Example
///
/// ```
/// # use beet_core::utils::CoalescingTrigger;
/// let trigger = CoalescingTrigger::default();
///
/// // First caller wins the lock
/// assert!(trigger.start());
///
/// // Second caller defers — sets dirty, does not run
/// assert!(!trigger.start());
///
/// // Finish: dirty flag detected, signals caller to re-run
/// assert!(trigger.finish());
///
/// // Finish: nothing pending, in-progress cleared
/// assert!(!trigger.finish());
/// ```
///
/// [`start`]: CoalescingTrigger::start
/// [`finish`]: CoalescingTrigger::finish
#[derive(Debug, Default, Clone)]
pub struct CoalescingTrigger(Arc<Mutex<CoalescingTriggerInner>>);

#[derive(Debug, Default)]
struct CoalescingTriggerInner {
	/// Another write was requested while one was in-flight.
	dirty: bool,
	/// A write is currently running.
	in_progress: bool,
}

impl CoalescingTrigger {
	/// Attempt to begin an operation.
	///
	/// Returns `true` if the caller should run the operation.
	/// Returns `false` if an operation is already in-flight; the dirty flag
	/// is set so the current operation will retry afterward.
	pub fn start(&self) -> bool {
		let mut inner = self.0.lock().unwrap();
		if inner.in_progress {
			inner.dirty = true;
			false
		} else {
			inner.in_progress = true;
			true
		}
	}

	/// Called after completing an operation.
	///
	/// Returns `true` if dirty — the caller should run the operation again.
	/// Returns `false` if nothing is pending; `in_progress` is cleared.
	pub fn finish(&self) -> bool {
		let mut inner = self.0.lock().unwrap();
		if inner.dirty {
			// clear dirty and stay in_progress for the next iteration
			inner.dirty = false;
			true
		} else {
			inner.in_progress = false;
			false
		}
	}

	/// Runs the provided function, coalescing concurrent calls into retries as needed.
	///
	/// Only one call runs in-flight at a time. If called while a call is already
	/// in-flight, the dirty flag is set and the in-flight call will re-run once
	/// after it finishes — regardless of how many extra calls arrive.
	///
	/// If the function returns an error, the trigger will be left in a consistent state
	/// and subsequent calls will still trigger retries as expected.
	pub async fn run_flush(&self, func: impl AsyncFn() -> Result) -> Result {
		// If a write is already in-flight, the dirty flag is set for a retry
		if !self.start() {
			return Ok(());
		}
		// Drive until no pending dirty requests remain
		loop {
			func().await?;
			if !self.finish() {
				break;
			}
		}
		Ok(())
	}
	/// Blocking version of [`run_flush`].
	pub fn run_flush_blocking(&self, func: impl Fn() -> Result) -> Result {
		// If a write is already in-flight, the dirty flag is set for a retry
		if !self.start() {
			return Ok(());
		}
		// Drive until no pending dirty requests remain
		loop {
			func()?;
			if !self.finish() {
				break;
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;

	#[test]
	fn starts_when_idle() {
		let trigger = CoalescingTrigger::default();
		trigger.start().xpect_true();
	}

	#[test]
	fn second_start_is_deferred() {
		let trigger = CoalescingTrigger::default();
		trigger.start(); // in-flight
		trigger.start().xpect_false(); // deferred, dirty set
	}

	#[test]
	fn finish_without_dirty_clears_in_progress() {
		let trigger = CoalescingTrigger::default();
		trigger.start();
		trigger.finish().xpect_false(); // nothing pending, done
		// in_progress is clear — can start again
		trigger.start().xpect_true();
	}

	#[test]
	fn finish_with_dirty_signals_retry() {
		let trigger = CoalescingTrigger::default();
		trigger.start();
		trigger.start(); // sets dirty
		trigger.finish().xpect_true(); // dirty -> retry
		trigger.finish().xpect_false(); // no dirty -> done
	}

	#[test]
	fn coalesces_many_requests_into_one_retry() {
		let trigger = CoalescingTrigger::default();
		trigger.start(); // in-flight

		// Many concurrent requests all collapse into a single dirty flag
		trigger.start().xpect_false();
		trigger.start().xpect_false();
		trigger.start().xpect_false();

		// Exactly one retry is needed — not three
		trigger.finish().xpect_true(); // retry
		trigger.finish().xpect_false(); // done
	}

	#[test]
	fn full_coalescing_loop() {
		use std::sync::atomic::AtomicU32;
		use std::sync::atomic::Ordering;

		let trigger = CoalescingTrigger::default();
		let write_count = AtomicU32::new(0);

		// Simulate: start -> 3 requests arrive -> finish loop
		trigger.start().xpect_true();
		trigger.start().xpect_false(); // dirty
		trigger.start().xpect_false(); // already dirty, no-op

		// Drive the write loop
		loop {
			write_count.fetch_add(1, Ordering::Relaxed);
			if !trigger.finish() {
				break;
			}
		}

		// Despite 3 requests, only 2 writes ran: the initial + one retry
		write_count.load(Ordering::Relaxed).xpect_eq(2);
	}
}
