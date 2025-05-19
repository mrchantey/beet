use extend::ext;
use std::time::Duration;

#[ext]
pub impl Duration {
	fn from_hms(hour: u64, minute: u64, second: u64) -> Duration {
		Duration::from_secs(hour * 3600 + minute * 60 + second)
	}
}
