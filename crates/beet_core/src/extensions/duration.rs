use crate::prelude::Duration;
use extend::ext;

/// Extension trait for [`Duration`] providing convenient constructors.
#[ext]
pub impl Duration {
	/// Creates a duration from hours, minutes, and seconds.
	///
	/// # Examples
	///
	/// ```
	/// use beet_core::prelude::*;
	///
	/// let duration = Duration::from_hms(1, 30, 45);
	/// assert_eq!(duration.as_secs(), 5445); // 1*3600 + 30*60 + 45
	/// ```
	fn from_hms(hour: u64, minute: u64, second: u64) -> Duration {
		Duration::from_secs(hour * 3600 + minute * 60 + second)
	}
}
