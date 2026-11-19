//! Stalls: a test holding the runner's one thread.
//!
//! Every test runs on the runner world's thread, async tests interleaved as
//! local tasks, so a test that blocks in one poll (CPU bound work never taken
//! off the executor, a blocking call) starves every other test in flight.
//! The test that then times out is usually the starved one rather than the
//! one stalling, so a timeout's report names every stall in its window.
use crate::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

/// One test's hold on the runner's thread, at least
/// [`RunnerStalls::THRESHOLD`] in a single call or poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerStall {
	/// The test that held the thread.
	pub test: String,
	/// How long it held it, summed over its stalls in the window.
	pub held: Duration,
}

/// The suite's log of [`RunnerStall`]s, shared with the test futures that
/// record into it.
#[derive(Debug, Default, Clone, Resource)]
pub(crate) struct RunnerStalls(Arc<Mutex<Vec<(Instant, RunnerStall)>>>);

impl RunnerStalls {
	/// The shortest hold logged: the most one runner frame adds to a test's
	/// timeout clock, since bevy's `Time` clamps a frame's delta there, so a
	/// longer stall is time the clock never sees.
	pub const THRESHOLD: Duration = Duration::from_millis(250);

	/// Run `func` as `test`, logging it when it holds the thread past
	/// [`Self::THRESHOLD`].
	pub fn time<T>(&self, test: &str, func: impl FnOnce() -> T) -> T {
		let start = Instant::now();
		let out = func();
		self.record(test, start.elapsed());
		out
	}

	/// `fut` with every poll timed as `test`.
	pub fn watch<F: Future>(&self, test: &str, fut: F) -> Watched<F> {
		Watched {
			stalls: self.clone(),
			test: test.to_string(),
			fut: Box::pin(fut),
		}
	}

	/// Every test that held the thread since `start`, longest first.
	pub fn since(&self, start: Instant) -> Vec<RunnerStall> {
		let mut held = Vec::<RunnerStall>::new();
		for (_, stall) in self
			.0
			.lock()
			.unwrap()
			.iter()
			.filter(|(end, _)| *end >= start)
		{
			match held.iter_mut().find(|entry| entry.test == stall.test) {
				Some(entry) => entry.held += stall.held,
				None => held.push(stall.clone()),
			}
		}
		held.sort_by(|a, b| b.held.cmp(&a.held));
		held
	}

	fn record(&self, test: &str, held: Duration) {
		if held >= Self::THRESHOLD {
			self.0.lock().unwrap().push((Instant::now(), RunnerStall {
				test: test.to_string(),
				held,
			}));
		}
	}
}

/// A test future whose every poll is timed into [`RunnerStalls`].
pub(crate) struct Watched<F> {
	stalls: RunnerStalls,
	test: String,
	fut: Pin<Box<F>>,
}

impl<F: Future> Future for Watched<F> {
	type Output = F::Output;
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
		let this = &mut *self;
		let start = Instant::now();
		let out = this.fut.as_mut().poll(cx);
		this.stalls.record(&this.test, start.elapsed());
		out
	}
}
