//! Copied from
//! https://github.com/yoshuawuyts/exponential-backoff
//! MIT/APACHE
//! - added stream
//!
//!
//! An exponential backoff generator with jitter. Serves as a building block to
//! implement custom retry functions.
//!
//! # Why?
//! When an network requests times out, often the best way to solve it is to try
//! again. But trying again straight away might at best cause some network overhead,
//! and at worst a full fledged DDOS. So we have to be responsible about it.
//!
//! A good explanation of retry strategies can be found on the [Stripe
//! blog](https://stripe.com/blog/idempotency).
//!
//! # Usage
//! Here we try and read a file from disk, and try again if it fails. A more
//! realistic scenario would probably to perform an HTTP request, but the approach
//! should be similar.
//!
//! ```rust
//! # fn retry() -> std::io::Result<()> {
//! use beet_utils::prelude::*;
//! use std::{fs, thread, time::Duration};
//!
//! let attempts = 3;
//! let min = Duration::from_millis(100);
//! let max = Duration::from_secs(10);
//!
//! for duration in Backoff::new(attempts, min, max) {
//!     match fs::read_to_string("README.md") {
//!         Ok(s) => {
//!             println!("{}", s);
//!             break;
//!         }
//!         Err(err) => match duration {
//!             Some(duration) => thread::sleep(duration),
//!             None => return Err(err),
//!         }
//!     }
//! }
//! # Ok(()) }
//! ```
//! Async retry with a stream
//!
//! This yields immediately for the first attempt, then sleeps between subsequent attempts
//! using the backoff policy.
//!
//! ```no_run
//! use beet_utils::utils::Backoff;
//! use futures::StreamExt;
//!
//! async fn try_operation() -> Result<(), ()> {
//!     // your fallible async work here
//!     Ok(())
//! }
//!
//! # async fn example() {
//! let mut stream = Backoff::default().stream();
//! while let Some((attempt, _duration)) = stream.next().await {
//!     if try_operation().await.is_ok() {
//!         break;
//!     }
//!     // The stream will sleep according to the backoff policy before yielding the next attempt.
//!     let _ = attempt; // 1-based attempt count
//! }
//! # }
//! ```
use std::time::Duration;

/// Exponential backoff type.
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
	/// use beet_utils::prelude::*;
	/// use std::time::Duration;
	///
	/// let backoff = Backoff::new(3, Duration::from_millis(100), Duration::from_secs(10));
	/// assert_eq!(backoff.max_attempts(), 3);
	/// assert_eq!(backoff.min(), &Duration::from_millis(100));
	/// assert_eq!(backoff.max(), &Duration::from_secs(10));
	/// ```
	///
	/// With no max duration (sets it to 584,942,417,355 years):
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	/// use std::time::Duration;
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
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	/// use std::time::Duration;
	///
	/// let mut backoff = Backoff::default();
	/// assert_eq!(backoff.min(), &Duration::from_millis(100));
	/// ```
	pub fn min(&self) -> &Duration { &self.min }

	/// Set the min duration.
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	/// use std::time::Duration;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_min(Duration::from_millis(50));
	/// assert_eq!(backoff.min(), &Duration::from_millis(50));
	/// ```
	#[inline]
	pub fn set_min(&mut self, min: Duration) { self.min = min; }

	/// Get the max duration
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	/// use std::time::Duration;
	///
	/// let mut backoff = Backoff::default();
	/// assert_eq!(backoff.max(), &Duration::from_secs(10));
	/// ```
	pub fn max(&self) -> &Duration { &self.max }

	/// Set the max duration.
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	/// use std::time::Duration;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_max(Duration::from_secs(30));
	/// assert_eq!(backoff.max(), &Duration::from_secs(30));
	/// ```
	#[inline]
	pub fn set_max(&mut self, max: Duration) { self.max = max; }

	/// Get the maximum number of attempts
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// assert_eq!(backoff.max_attempts(), 3);
	/// ```
	pub fn max_attempts(&self) -> u32 { self.max_attempts }

	/// Set the maximum number of attempts.
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_max_attempts(5);
	/// assert_eq!(backoff.max_attempts(), 5);
	/// ```
	pub fn set_max_attempts(&mut self, max_attempts: u32) {
		self.max_attempts = max_attempts;
	}

	/// Get the jitter factor
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// assert_eq!(backoff.jitter(), 0.3);
	/// ```
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
	/// use beet_utils::prelude::*;
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
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// assert_eq!(backoff.factor(), 2);
	/// ```
	pub fn factor(&self) -> u32 { self.factor }

	/// Set the growth factor for each iteration of the backoff.
	///
	/// # Examples
	///
	/// ```rust
	/// use beet_utils::prelude::*;
	///
	/// let mut backoff = Backoff::default();
	/// backoff.set_factor(3);
	/// assert_eq!(backoff.factor(), 3);
	/// ```
	#[inline]
	pub fn set_factor(&mut self, factor: u32) { self.factor = factor; }

	/// Create an iterator.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_utils::utils::Backoff;
	/// use std::time::Duration;
	///
	/// let backoff = Backoff::new(3, Duration::from_millis(100), Duration::from_secs(10));
	/// let mut count = 0;
	/// for duration in backoff.iter() {
	///     // duration is Some(d) for attempts which, on failure, will back off before the next attempt,
	///     // and None for the final attempt (no further sleep).
	///     let _ = duration;
	///     count += 1;
	/// }
	/// assert_eq!(count, 3);
	/// ```
	#[inline]
	pub fn iter(&self) -> BackoffIter { BackoffIter::new(self.clone()) }
}

/// Implements the `IntoIterator` trait for borrowed `Backoff` instances.
///
/// # Examples
///
/// ```rust
/// use beet_utils::prelude::*;
/// use std::time::Duration;
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
	type Item = Option<Duration>;
	type IntoIter = BackoffIter;

	fn into_iter(self) -> Self::IntoIter { Self::IntoIter::new(self.clone()) }
}

/// Implements the `IntoIterator` trait for owned `Backoff` instances.
///
/// # Examples
///
/// ```rust
/// use beet_utils::prelude::*;
/// use std::time::Duration;
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
	type Item = Option<Duration>;
	type IntoIter = BackoffIter;

	fn into_iter(self) -> Self::IntoIter { Self::IntoIter::new(self) }
}

/// Implements the `Default` trait for `Backoff`.
///
/// # Examples
///
/// ```rust
/// use beet_utils::prelude::*;
/// use std::time::Duration;
///
/// let backoff = Backoff::default();
/// assert_eq!(backoff.max_attempts(), 3);
/// assert_eq!(backoff.min(), &Duration::from_millis(100));
/// assert_eq!(backoff.max(), &Duration::from_secs(10));
/// #[cfg(feature = "rand")]
/// assert_eq!(backoff.jitter(), 0.3);
/// assert_eq!(backoff.factor(), 2);
/// ```
impl Default for Backoff {
	fn default() -> Self {
		Self {
			max_attempts: 3,
			min: Duration::from_millis(100),
			max: Duration::from_secs(10),
			factor: 2,
			#[cfg(feature = "rand")]
			jitter: 0.3,
		}
	}
}
use std::iter;
/// An exponential backoff iterator.
#[derive(Debug, Clone)]
pub struct BackoffIter {
	inner: Backoff,
	#[cfg(feature = "rand")]
	rng: rand::rngs::ThreadRng,
	attempts: u32,
}

impl BackoffIter {
	pub(crate) fn new(inner: Backoff) -> Self {
		Self {
			attempts: 0,
			#[cfg(feature = "rand")]
			rng: rand::thread_rng(),
			inner,
		}
	}
}

impl iter::Iterator for BackoffIter {
	type Item = Option<Duration>;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		// Check whether we've exceeded the number of attempts,
		// or whether we're on our last attempt. We don't want to sleep after
		// the last attempt.
		if self.attempts == self.inner.max_attempts {
			return None;
		} else if self.attempts == self.inner.max_attempts - 1 {
			self.attempts = self.attempts.saturating_add(1);
			return Some(None);
		}

		// Create exponential duration.
		let exponent = self.inner.factor.saturating_pow(self.attempts);
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

		Some(Some(duration))
	}
}

/// A Stream that yields per attempt and sleeps between attempts according to Backoff.
///
/// The item is `(attempt_index, Option<Duration>)`:
/// - `attempt_index` starts at `1`.
/// - `Option<Duration>` is the backoff duration associated with this attempt as produced by the iterator:
///   - `Some(duration)` for attempts that, on failure, will back off before the next attempt,
///   - `None` for the final attempt (no further sleep).
///
/// Behavior:
/// - First attempt is yielded immediately (no pre-sleep).
/// - Between subsequent attempts the stream sleeps using `time_ext::sleep(duration).await`.
pub struct BackoffStream {
	iter: BackoffIter,
	// Duration for the attempt to be yielded next (as produced by BackoffIter).
	current: Option<Option<Duration>>,
	// 1-based attempt counter.
	attempt: u32,
	sleeper: Option<
		std::pin::Pin<Box<dyn core::future::Future<Output = ()> + 'static>>,
	>,
}

impl BackoffStream {
	fn new(inner: Backoff) -> Self {
		let mut iter = BackoffIter::new(inner);
		let current = iter.next();
		Self {
			iter,
			current,
			attempt: 0,
			sleeper: None,
		}
	}
}

impl Backoff {
	/// Create a Stream that yields `(attempt_index, Option<Duration>)` per attempt.
	///
	/// The stream yields the first attempt immediately (without sleeping). For subsequent
	/// attempts it sleeps according to the backoff policy before yielding. The `Option<Duration>`
	/// corresponds to the iterator's value for that attempt (`None` for the last attempt).
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[cfg(feature = "tokio")]
	/// # async fn demo() {
	/// use beet_utils::utils::Backoff;
	/// use futures::StreamExt;
	///
	/// let backoff = Backoff::default();
	/// let mut stream = backoff.stream();
	///
	/// while let Some((attempt, duration)) = stream.next().await {
	///     // Perform your operation for this attempt.
	///     // `duration` is the backoff that will be applied if this attempt fails.
	///     let _ = (attempt, duration);
	/// }
	/// # }
	/// ```
	pub fn stream(&self) -> BackoffStream { BackoffStream::new(self.clone()) }
}

impl futures::Stream for BackoffStream {
	type Item = (u32, Option<Duration>);

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
			Some(duration) => {
				// Increment attempt count and schedule the next sleep if needed.
				self.attempt = self.attempt.saturating_add(1);
				if let Some(d) = duration {
					// Schedule sleep before the next attempt is yielded.
					self.sleeper = Some(Box::pin(crate::time_ext::sleep(d)));
				}
				std::task::Poll::Ready(Some((self.attempt, duration)))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn iter_without_rand_deterministic() {
		let backoff = Backoff::new(
			3,
			Duration::from_millis(100),
			Duration::from_secs(10),
		);
		let mut it = backoff.iter();

		assert_eq!(it.next(), Some(Some(Duration::from_millis(100))));
		assert_eq!(it.next(), Some(Some(Duration::from_millis(200))));
		// Final attempt yields None to signal "last try, no sleep"
		assert_eq!(it.next(), Some(None));
		// Then the iterator is exhausted
		assert_eq!(it.next(), None);
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
		assert_eq!(it.next(), Some(Some(Duration::from_millis(500))));
		// 2nd: 1500ms -> clamped to 900ms
		assert_eq!(it.next(), Some(Some(Duration::from_millis(900))));
		// 3rd: would be bigger, still clamped
		assert_eq!(it.next(), Some(Some(Duration::from_millis(900))));
		// 4th: still clamped
		assert_eq!(it.next(), Some(Some(Duration::from_millis(900))));
		// 5th: final attempt, no sleep
		assert_eq!(it.next(), Some(None));
		// Exhausted
		assert_eq!(it.next(), None);
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

		let d1 = it.next().unwrap().unwrap();
		// jitter ~ +/- 30% (implementation yields up to ~29% due to integer math)
		assert!(
			d1 >= Duration::from_millis(70) && d1 <= Duration::from_millis(130)
		);

		let d2 = it.next().unwrap().unwrap();
		assert!(
			d2 >= Duration::from_millis(140)
				&& d2 <= Duration::from_millis(260)
		);

		// Final attempt: None means no sleep
		assert_eq!(it.next(), Some(None));
		assert_eq!(it.next(), None);
	}

	#[cfg(feature = "tokio")]
	#[tokio::test]
	async fn stream_sleeps_and_yields_attempts() {
		use futures::StreamExt;

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
		let start = std::time::Instant::now();
		let mut items = Vec::new();

		while let Some((attempt, duration)) = stream.next().await {
			println!("NEXT");
			items.push((attempt, duration));
		}

		// We should get exactly max_attempts yields, starting at attempt=1.
		assert_eq!(items.len(), 3);
		assert_eq!(items[0], (1, Some(Duration::from_millis(5))));
		assert_eq!(items[1], (2, Some(Duration::from_millis(10))));
		assert_eq!(items[2], (3, None));

		// With factor=2 and no jitter, total sleep time is 5ms + 10ms = 15ms (last attempt does not sleep).
		let elapsed = start.elapsed();
		assert!(elapsed >= Duration::from_millis(15));
	}
}
