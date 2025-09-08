use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;


static NUM_UPTIME_REQUESTS: AtomicUsize = AtomicUsize::new(0);
///
/// Handy uptime struct for use in axum state
/// Like all substates ensure that `FromRef` is implemented:
/// ```rust ignore
/// impl FromRef<AppState> for Uptime {
///	  fn from_ref(app_state: &AppState) -> Uptime { app_state.uptime.clone() }
///	}
/// ```
///
#[derive(Debug, Clone)]
pub struct Uptime {
	pub start: std::time::Instant,
}
impl Default for Uptime {
	fn default() -> Self { Self::new() }
}



impl Uptime {
	pub fn new() -> Self {
		Self {
			start: std::time::Instant::now(),
		}
	}
	pub fn incr_requests(&self) -> usize {
		NUM_UPTIME_REQUESTS.fetch_add(1, Ordering::SeqCst) + 1
	}

	pub fn stats(&self) -> String {
		let uptime = self.start.elapsed().as_secs();
		let requests = self.incr_requests();
		format!("Uptime: {} seconds, Requests: {}", uptime, requests)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::extract::State;
	use axum::routing::get;
	use http::StatusCode;
	use sweet::prelude::*;


	async fn uptime(State(uptime): State<Uptime>) -> (StatusCode, String) {
		(
			StatusCode::OK,
			uptime.start.elapsed().as_millis().to_string(),
		)
	}


	#[sweet::test]
	async fn works() {
		use std::time::Duration;

		let mut router = Router::new()
			.route("/", get(uptime))
			.with_state(Uptime::new());

		tokio::time::sleep(Duration::from_millis(10)).await;
		let time = router.oneshot_str("/").await.unwrap();
		let time: u64 = time.parse().unwrap();
		time.xpect().to_be_greater_or_equal_to(10);
		time.xpect().to_be_less_than(20);
	}
}
