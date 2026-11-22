use beet_core::prelude::*;

/// The device authorization grant's poll (RFC 8628): after the person is
/// shown the code and the verification url, ask for the token every
/// `interval` until it is granted, the code expires, or the server refuses.
/// A `slow_down` answer widens the interval by five seconds for good.
#[derive(Debug, Clone)]
pub struct DeviceGrant {
	interval: Duration,
	expires_in: Duration,
	slow_down_step: Duration,
}

/// One poll's answer, as the provider's client reads its token endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevicePoll<T> {
	/// The person approved: the grant's answer.
	Granted(T),
	/// Not yet approved (`authorization_pending`).
	Pending,
	/// Polling too fast (`slow_down`).
	SlowDown,
}

impl DeviceGrant {
	/// A poll every `interval` for at most `expires_in`, as the device
	/// authorization answered.
	pub fn new(interval: Duration, expires_in: Duration) -> Self {
		Self {
			interval,
			expires_in,
			slow_down_step: Duration::from_secs(5),
		}
	}

	/// Widen the interval by `step` on a `slow_down`, a test's zero.
	pub fn with_slow_down_step(mut self, step: Duration) -> Self {
		self.slow_down_step = step;
		self
	}

	/// Poll through `attempt` until it is granted, the code expires, or
	/// `attempt` errors (a denial, an expired code).
	pub async fn poll<T, F, Fut>(&self, mut attempt: F) -> Result<T>
	where
		F: FnMut() -> Fut,
		Fut: Future<Output = Result<DevicePoll<T>>>,
	{
		let start = Instant::now();
		let mut interval = self.interval;
		loop {
			time_ext::sleep(interval).await;
			match attempt().await? {
				DevicePoll::Granted(granted) => return Ok(granted),
				DevicePoll::Pending => {}
				DevicePoll::SlowDown => interval += self.slow_down_step,
			}
			if start.elapsed() >= self.expires_in {
				bevybail!(
					"the sign in code expired before it was approved: start the \
					login again"
				);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Pending and slow down keep polling, the grant ends it.
	#[beet_core::test]
	async fn polls_until_granted() {
		let mut answers = vec![
			DevicePoll::Pending,
			DevicePoll::SlowDown,
			DevicePoll::Granted(7),
		]
		.into_iter();
		let mut polls = 0;
		DeviceGrant::new(Duration::ZERO, Duration::from_secs(60))
			.with_slow_down_step(Duration::ZERO)
			.poll(|| {
				polls += 1;
				let answer = answers.next().unwrap();
				async move { Ok(answer) }
			})
			.await
			.unwrap()
			.xpect_eq(7);
		polls.xpect_eq(3);
	}

	/// A code nobody approves expires.
	#[beet_core::test]
	async fn expires() {
		DeviceGrant::new(Duration::from_millis(5), Duration::from_millis(20))
			.poll(|| async { Ok(DevicePoll::<()>::Pending) })
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("expired");
	}
}
