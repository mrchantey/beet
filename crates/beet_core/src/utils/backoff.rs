// Copied from
// https://github.com/yoshuawuyts/exponential-backoff
// MIT/APACHE
// - added stream
//
use crate::prelude::*;

use std::pin::Pin;
use std::time::Duration;

impl Default for Backoff {
	fn default() -> Self {
		Self {
			max_attempts: 3,
			min: Duration::from_millis(100),
			max: Duration::from_secs(4),
			factor: 2,
			#[cfg(feature = "rand")]
			jitter: 0.3,
		}
	}
}

/// An exponential backoff generator with jitter. Serves as a building block to
/// implement custom retry functions.
///
/// # Why?
/// When an network requests times out, often the best way to solve it is to try
/// again. But trying again straight away might at best cause some network overhead,
/// and at worst a full fledged DDOS. So we have to be responsible about it.
///
/// A good explanation of retry strategies can be found on the [Stripe
/// blog](https://stripe.com/blog/idempotency).
///
/// # Usage
/// Primary usage is via `Backoff::retry` and `Backoff::retry_async`.
///
/// Synchronous:
///
/// ```no_run
/// use beet_core::utils::Backoff;
/// use std::time::Duration;
///
/// # fn main() -> Result<(), std::io::Error> {
/// let contents = Backoff::default().retry(|frame| {
///     match std::fs::read_to_string("README.md") {
///         Ok(s) => Ok(s),
///         Err(e) => {
///             // Returning Err will cause retry according to the backoff policy.
///             // `frame.next_attempt` holds the sleep duration before the next try,
///             // or None if this was the final attempt.
///             Err(e)
///         }
///     }
/// })?;
/// println!("{}", contents);
/// # Ok(()) }
/// ```
///
/// If you need lower-level control, you can also use [`Self::iter()`] or [`Self::stream()`].
#[derive(Debug, Clone)]
pub struct Backoff {
	max_attempts: u32,
	min: Duration,
	max: Duration,
	#[cfg(feature = "rand")]
	jitter: f32,
	factor: u32,
}
impl Backoff {
	/// Create a new instance of `Backoff`.
	///
	/// # Examples
	///
	/// With an explicit max duration:
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// # use std::time::Duration;
	///
	/// let backoff = Backoff::new(3, Duration::from_millis(100), Duration::from_secs(10));
	/// assert_eq!(backoff.max_attempts(), 3);
	/// assert_eq!(backoff.min(), &Duration::from_millis(100));
	/// assert_eq!(backoff.max(), &Duration::from_secs(10));
	/// ```
	///
	/// With no max duration, defaults to Duration::MAX (many many years)
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// # use std::time::Duration;
	///
	/// let backoff = Backoff::new(5, Duration::from_millis(50), None);
	/// # assert_eq!(backoff.max_attempts(), 5);
	/// # assert_eq!(backoff.min(), &Duration::from_millis(50));
	/// assert_eq!(backoff.max(), &Duration::MAX);
	/// ```
	#[inline]
	pub fn new(
		max_attempts: u32,
		min: Duration,
		max: impl Into<Option<Duration>>,
	) -> Self {
		Self {
			max_attempts,
			min,
			max: max.into().unwrap_or(Duration::MAX),
			#[cfg(feature = "rand")]
			jitter: 0.3,
			factor: 2,
		}
	}

	/// Get the min duration
	pub fn min(&self) -> &Duration { &self.min }

	/// Set the min duration.
	///
	/// # Examples
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// # use std::time::Duration;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_min(Duration::from_millis(50));
	/// assert_eq!(backoff.min(), &Duration::from_millis(50));
	/// ```
	#[inline]
	pub fn set_min(&mut self, min: Duration) { self.min = min; }

	/// Get the max duration
	pub fn max(&self) -> &Duration { &self.max }

	/// Set the max duration.
	///
	/// # Examples
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// # use std::time::Duration;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_max(Duration::from_secs(30));
	/// assert_eq!(backoff.max(), &Duration::from_secs(30));
	/// ```
	#[inline]
	pub fn set_max(&mut self, max: Duration) { self.max = max; }

	/// Get the maximum number of attempts
	pub fn max_attempts(&self) -> u32 { self.max_attempts }

	/// Set the maximum number of attempts.
	///
	/// # Examples
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_max_attempts(5);
	/// assert_eq!(backoff.max_attempts(), 5);
	/// ```
	pub fn set_max_attempts(&mut self, max_attempts: u32) {
		self.max_attempts = max_attempts;
	}

	/// Get the jitter factor
	#[cfg(feature = "rand")]
	pub fn jitter(&self) -> f32 { self.jitter }

	/// Set the amount of jitter per backoff.
	///
	/// # Panics
	///
	/// This method panics if a number smaller than `0` or larger than `1` is
	/// provided.
	///
	/// # Examples
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_jitter(0.3);  // default value
	/// backoff.set_jitter(0.0);  // min value
	/// backoff.set_jitter(1.0);  // max value
	/// ```
	#[inline]
	#[cfg(feature = "rand")]
	pub fn set_jitter(&mut self, jitter: f32) {
		assert!(
			jitter >= 0f32 && jitter <= 1f32,
			"<exponential-backoff>: jitter must be between 0 and 1."
		);
		self.jitter = jitter;
	}

	/// Get the growth factor
	pub fn factor(&self) -> u32 { self.factor }

	/// Set the growth factor for each iteration of the backoff.
	///
	/// # Examples
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_factor(3);
	/// assert_eq!(backoff.factor(), 3);
	/// ```
	#[inline]
	pub fn set_factor(&mut self, factor: u32) { self.factor = factor; }

	/// Builder: set the maximum number of attempts.
	#[inline]
	pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
		self.set_max_attempts(max_attempts);
		self
	}

	/// Builder: set the minimum backoff duration.
	#[inline]
	pub fn with_min(mut self, min: Duration) -> Self {
		self.set_min(min);
		self
	}

	/// Builder: set the maximum backoff duration. Passing `None` sets it to `Duration::MAX`.
	#[inline]
	pub fn with_max(mut self, max: Duration) -> Self {
		self.set_max(max);
		self
	}

	/// Builder: set the jitter factor (0.0..=1.0).
	#[cfg(feature = "rand")]
	#[inline]
	pub fn with_jitter(mut self, jitter: f32) -> Self {
		self.set_jitter(jitter);
		self
	}

	/// Builder: set the exponential growth factor.
	#[inline]
	pub fn with_factor(mut self, factor: u32) -> Self {
		self.set_factor(factor);
		self
	}

	/// Create an iterator.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use beet_core::utils::Backoff;
	/// # use std::time::Duration;
	///
	/// let backoff = Backoff::new(3, Duration::from_millis(100), Duration::from_secs(10));
	/// let mut count = 0;
	/// for frame in backoff.iter() {
	///     // frame.next_attempt is Some(d) for attempts which, on failure, will back off before the next attempt,
	///     // and None for the final attempt (no further sleep). frame.attempt_index is zero-based.
	///     let _ = frame;
	///     count += 1;
	/// }
	/// assert_eq!(count, 3);
	/// ```
	#[inline]
	pub fn iter(&self) -> BackoffIter { BackoffIter::new(self.clone()) }
	/// Create a Stream that yields `BackoffFrame` per attempt.
	///
	/// The stream yields the first attempt immediately (without sleeping). For subsequent
	/// attempts it sleeps according to the backoff policy before yielding. `BackoffFrame::next_attempt`
	/// is the backoff duration for the following attempt (`None` for the last attempt).
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn demo() {
	/// # use beet_core::utils::Backoff;
	/// # use futures::StreamExt;
	///
	/// let backoff = Backoff::default();
	/// let mut stream = backoff.stream();
	///
	/// while let Some(_frame) = stream.next().await {
	///     // Perform your operation for this attempt.
	///     // `frame.next_attempt` is the backoff that will be applied if this attempt fails.
	/// }
	/// # }
	/// ```
	pub fn stream(&self) -> BackoffStream { BackoffStream::new(self.clone()) }
	/// Retry a synchronous operation using this backoff.
	///
	/// (synchronous sleep is unsupported on wasm, see [`Self::retry_async`])
	///
	/// The closure is called for each attempt with a `BackoffFrame`, and should return
	/// `Ok(T)` on success or `Err(E)` to trigger another attempt (until the final attempt).
	/// Between attempts this method sleeps according to the backoff policy.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_core::utils::Backoff;
	/// use std::time::Duration;
	///
	/// # fn main() -> Result<(), ()> {
	/// let backoff = Backoff::new(3, Duration::from_millis(10), Duration::from_millis(100));
	/// let _value = backoff.retry(|_frame| -> Result<&'static str, ()> {
	///     // do work...
	///     Err(())
	/// })?;
	/// # Ok(()) }
	/// ```
	#[cfg(not(target_arch = "wasm32"))]
	pub fn retry<T, E, F>(&self, mut op: F) -> Result<T, E>
	where
		F: FnMut(BackoffFrame) -> Result<T, E>,
	{
		for frame in self.iter() {
			match op(frame) {
				Ok(v) => return Ok(v),
				Err(err) => match frame.next_attempt {
					Some(d) => std::thread::sleep(d),
					None => return Err(err),
				},
			}
		}
		unreachable!("Backoff::iter must yield at least one frame")
	}

	/// Retry an asynchronous operation using this backoff.
	///
	/// The closure is called for each attempt with a `BackoffFrame`, and should return
	/// a Future resolving to `Ok(T)` on success or `Err(E)` to trigger another attempt.
	/// The backoff sleeps between attempts using the underlying stream.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_core::utils::Backoff;
	/// use std::time::Duration;
	///
	/// # async fn demo() -> Result<(), ()> {
	/// let backoff = Backoff::new(3, Duration::from_millis(10), Duration::from_millis(100));
	/// let _ = backoff.retry_async(|_frame| async {
	///     Err::<(), ()>(())
	/// }).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn retry_async<T, E, Fut, F>(&self, mut op: F) -> Result<T, E>
	where
		F: FnMut(BackoffFrame) -> Fut,
		Fut: core::future::Future<Output = Result<T, E>>,
	{
		let mut stream = self.stream();
		while let Some(frame) = stream.next().await {
			match op(frame).await {
				Ok(v) => return Ok(v),
				Err(err) => {
					if frame.is_final() {
						return Err(err);
					}
				}
			}
		}
		unreachable!("Backoff::stream must yield at least one frame")
	}
}

/// Implements the `IntoIterator` trait for borrowed `Backoff` instances.
///
/// # Examples
///
/// ```rust
/// # use beet_core::prelude::*;
/// # use std::time::Duration;
///
/// let backoff = Backoff::default();
/// let mut count = 0;
///
/// for duration in &backoff {
///     count += 1;
///     if count > 1 {
///         break;
///     }
/// }
/// ```
impl<'b> IntoIterator for &'b Backoff {
	type Item = BackoffFrame;
	type IntoIter = BackoffIter;

	fn into_iter(self) -> Self::IntoIter { Self::IntoIter::new(self.clone()) }
}

/// Implements the `IntoIterator` trait for owned `Backoff` instances.
///
/// # Examples
///
/// ```rust
/// # use beet_core::prelude::*;
/// # use std::time::Duration;
///
/// let backoff = Backoff::default();
/// let mut count = 0;
///
/// for duration in backoff {
///     count += 1;
///     if count > 1 {
///         break;
///     }
/// }
/// ```
impl IntoIterator for Backoff {
	type Item = BackoffFrame;
	type IntoIter = BackoffIter;

	fn into_iter(self) -> Self::IntoIter { Self::IntoIter::new(self) }
}

use std::iter;
/// An exponential backoff iterator.
#[derive(Debug, Clone)]
pub struct BackoffIter {
	inner: Backoff,
	#[cfg(feature = "rand")]
	rng: rand::rngs::StdRng,
	attempts: u32,
}

impl BackoffIter {
	pub(crate) fn new(inner: Backoff) -> Self {
		Self {
			attempts: 0,
			#[cfg(feature = "rand")]
			rng: {
				use rand::SeedableRng;
				rand::rngs::StdRng::from_entropy()
			},
			inner,
		}
	}
}

impl iter::Iterator for BackoffIter {
	type Item = BackoffFrame;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		// Check whether we've exceeded the number of attempts,
		// or whether we're on our last attempt. We don't want to sleep after
		// the last attempt.
		if self.attempts == self.inner.max_attempts {
			return None;
		}

		// Zero-based attempt index for this yield.
		let attempt_index = self.attempts;

		// If this is the last attempt, yield with no further sleep.
		if attempt_index == self.inner.max_attempts - 1 {
			self.attempts = self.attempts.saturating_add(1);
			return Some(BackoffFrame {
				attempt_index,
				next_attempt: None,
			});
		}

		// Create exponential duration.
		let exponent = self.inner.factor.saturating_pow(attempt_index);
		let mut duration = self.inner.min.saturating_mul(exponent);

		// Increment the attempts counter.
		self.attempts = self.attempts.saturating_add(1);

		// Apply jitter. Uses multiples of 100 to prevent relying on floats.
		//
		// We put this in a conditional block because the `fastrand` crate
		// doesn't like `0..0` inputs, and dividing by zero is also not a good
		// idea.

		#[cfg(feature = "rand")]
		if self.inner.jitter != 0.0 {
			use rand::Rng;
			let jitter_factor = (self.inner.jitter * 100f32) as u32;
			let random = self.rng.gen_range(0..jitter_factor * 2);
			let mut duration = duration.saturating_mul(100);
			if random < jitter_factor {
				let jitter = duration.saturating_mul(random) / 100;
				duration = duration.saturating_sub(jitter);
			} else {
				let jitter = duration.saturating_mul(random / 2) / 100;
				duration = duration.saturating_add(jitter);
			};
			duration /= 100;
		}

		// Make sure it doesn't exceed upper / lower bounds.
		duration = duration.clamp(self.inner.min, self.inner.max);

		Some(BackoffFrame {
			attempt_index,
			next_attempt: Some(duration),
		})
	}
}

/// Frame yielded by BackoffStream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffFrame {
	/// 0-based attempt count for this yield.
	pub attempt_index: u32,
	/// Backoff duration that will be awaited before the next attempt, or None for the final attempt.
	pub next_attempt: Option<Duration>,
}

impl std::fmt::Display for BackoffFrame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.next_attempt {
			Some(d) => write!(
				f,
				"Attempt: {}, Next: {}ms",
				self.attempt_index,
				d.as_millis()
			),
			None => write!(f, "Attempt: {}, Next: None", self.attempt_index),
		}
	}
}

impl BackoffFrame {
	/// Returns true if this is the final attempt.
	pub fn is_final(&self) -> bool { self.next_attempt.is_none() }
}

/// A Stream that yields a [`BackoffFrame`] per attempt and sleeps between attempts according to Backoff.
/// Behavior:
/// - First attempt is yielded immediately (no pre-sleep).
/// - Between subsequent attempts the stream sleeps using `time_ext::sleep(duration).await`.
pub struct BackoffStream {
	iter: BackoffIter,
	// Frame for the attempt to be yielded next (as produced by BackoffIter).
	current: Option<BackoffFrame>,
	#[cfg(target_arch = "wasm32")]
	sleeper: Option<Pin<Box<dyn Future<Output = ()> + 'static>>>,
	#[cfg(not(target_arch = "wasm32"))]
	sleeper: Option<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
}

impl BackoffStream {
	fn new(inner: Backoff) -> Self {
		let mut iter = BackoffIter::new(inner);
		let current = iter.next();
		Self {
			iter,
			current,
			sleeper: None,
		}
	}
}

impl futures::Stream for BackoffStream {
	type Item = BackoffFrame;

	fn poll_next(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		// If a sleep is in-flight, drive it to completion first.
		if let Some(fut) = self.sleeper.as_mut() {
			match fut.as_mut().poll(cx) {
				std::task::Poll::Pending => return std::task::Poll::Pending,
				std::task::Poll::Ready(()) => {
					// Completed the scheduled sleep. Prepare next attempt's current.
					self.sleeper = None;
					self.current = self.iter.next();
				}
			}
		}

		// Yield the next attempt if we have one.
		match self.current.take() {
			None => std::task::Poll::Ready(None),
			Some(frame) => {
				if let Some(d) = frame.next_attempt {
					// Schedule sleep before the next attempt is yielded.
					self.sleeper = Some(Box::pin(crate::time_ext::sleep(d)));
				}
				std::task::Poll::Ready(Some(frame))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;
	use sweet::prelude::*;

	#[test]
	fn iter_without_rand_deterministic() {
		let backoff = Backoff::new(
			3,
			Duration::from_millis(100),
			Duration::from_secs(10),
		);
		let mut it = backoff.iter();

		let f0 = it.next().unwrap();
		f0.attempt_index.xpect_eq(0);
		f0.next_attempt.xpect_eq(Some(Duration::from_millis(100)));
		let f1 = it.next().unwrap();
		f1.attempt_index.xpect_eq(1);
		f1.next_attempt.xpect_eq(Some(Duration::from_millis(200)));
		// Final attempt yields None to signal "last try, no sleep"
		let f2 = it.next().unwrap();
		f2.attempt_index.xpect_eq(2);
		f2.next_attempt.xpect_none();
		// Then the iterator is exhausted
		it.next().xpect_none();
	}

	#[test]
	fn clamps_to_max() {
		let mut backoff = Backoff::new(
			5,
			Duration::from_millis(500),
			Duration::from_millis(900),
		);
		backoff.set_factor(3);
		let mut it = backoff.iter();

		// 1st: 500ms
		let f0 = it.next().unwrap();
		f0.attempt_index.xpect_eq(0);
		f0.next_attempt.xpect_eq(Some(Duration::from_millis(500)));
		// 2nd: 1500ms -> clamped to 900ms
		let f1 = it.next().unwrap();
		f1.attempt_index.xpect_eq(1);
		f1.next_attempt.xpect_eq(Some(Duration::from_millis(900)));
		// 3rd: would be bigger, still clamped
		let f2 = it.next().unwrap();
		f2.attempt_index.xpect_eq(2);
		f2.next_attempt.xpect_eq(Some(Duration::from_millis(900)));
		// 4th: still clamped
		let f3 = it.next().unwrap();
		f3.attempt_index.xpect_eq(3);
		f3.next_attempt.xpect_eq(Some(Duration::from_millis(900)));
		// 5th: final attempt, no sleep
		let f4 = it.next().unwrap();
		f4.attempt_index.xpect_eq(4);
		f4.next_attempt.xpect_eq(None);
		// Exhausted
		it.next().xpect_eq(None);
	}

	#[test]
	#[cfg(not(target_arch = "wasm32"))]
	fn retry_succeeds_after_failures() {
		use std::sync::atomic::AtomicU32;
		use std::sync::atomic::Ordering;

		let counter = AtomicU32::new(0);
		#[allow(unused_mut)]
		let mut backoff = Backoff::new(
			3,
			Duration::from_millis(1),
			Duration::from_millis(10),
		);
		#[cfg(feature = "rand")]
		{
			backoff.set_jitter(0.0);
		}

		let res: Result<u32, ()> = backoff.retry(|_frame| {
			let c = counter.fetch_add(1, Ordering::SeqCst);
			if c >= 2 { Ok(c) } else { Err(()) }
		});

		res.unwrap().xpect_eq(2);
		(counter.load(Ordering::SeqCst) >= 3).xpect_true();
	}

	#[sweet::test]
	async fn retry_async_succeeds_after_failures() {
		use std::sync::atomic::AtomicU32;
		use std::sync::atomic::Ordering;
		let counter = std::sync::Arc::new(AtomicU32::new(0));
		#[allow(unused_mut)]
		let mut backoff = Backoff::new(
			3,
			Duration::from_millis(5),
			Duration::from_millis(100),
		);
		#[cfg(feature = "rand")]
		{
			backoff.set_jitter(0.0);
		}

		let start = web_time::Instant::now();
		let result = backoff
			.retry_async({
				let counter = counter.clone();
				move |_frame| {
					let c = counter.fetch_add(1, Ordering::SeqCst);
					async move {
						if c >= 2 {
							Ok::<u32, ()>(c)
						} else {
							Err::<u32, ()>(())
						}
					}
				}
			})
			.await;

		result.unwrap().xpect_eq(2);
		// With factor=2 and no jitter, before success on attempt 2 (0-based),
		// total sleep time is 5ms + 10ms = 15ms.
		let elapsed = start.elapsed();
		(elapsed >= Duration::from_millis(15)).xpect_true();
	}

	#[cfg(feature = "rand")]
	#[test]
	fn iter_with_rand_in_range() {
		let backoff = Backoff::new(
			3,
			Duration::from_millis(100),
			Duration::from_secs(10),
		);
		let mut it = backoff.iter();

		let f0 = it.next().unwrap();
		let d1 = f0.next_attempt.unwrap();
		// jitter ~ +/- 30% (implementation yields up to ~29% due to integer math)
		(d1 >= Duration::from_millis(70) && d1 <= Duration::from_millis(130))
			.xpect_true();

		let f1 = it.next().unwrap();
		let d2 = f1.next_attempt.unwrap();
		(d2 >= Duration::from_millis(140) && d2 <= Duration::from_millis(260))
			.xpect_true();

		// Final attempt: None means no sleep
		let f2 = it.next().unwrap();
		f2.next_attempt.xpect_eq(None);
		it.next().xpect_eq(None);
	}

	#[sweet::test]
	async fn stream_sleeps_and_yields_attempts() {
		#[allow(unused_mut)]
		let mut backoff = Backoff::new(
			3,
			Duration::from_millis(5),
			Duration::from_millis(100),
		);
		// Disable jitter to make timing deterministic when "rand" is enabled.
		#[cfg(feature = "rand")]
		{
			backoff.set_jitter(0.0);
		}

		let mut stream = backoff.stream();
		let start = web_time::Instant::now();
		let mut items = Vec::new();

		while let Some(frame) = stream.next().await {
			println!("NEXT");
			items.push(frame);
		}

		// We should get exactly max_attempts yields, starting at attempt=0.
		items.len().xpect_eq(3);
		items[0].xpect_eq(BackoffFrame {
			attempt_index: 0,
			next_attempt: Some(Duration::from_millis(5)),
		});
		items[1].xpect_eq(BackoffFrame {
			attempt_index: 1,
			next_attempt: Some(Duration::from_millis(10)),
		});
		items[2].xpect_eq(BackoffFrame {
			attempt_index: 2,
			next_attempt: None,
		});

		// With factor=2 and no jitter, total sleep time is 5ms + 10ms = 15ms (last attempt does not sleep).
		let elapsed = start.elapsed();
		(elapsed >= Duration::from_millis(15)).xpect_true();
	}
}
