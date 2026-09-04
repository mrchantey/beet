use crate::prelude::*;

/// A wall-clock instant, stored as the signed milliseconds since the Unix epoch,
/// and the owner of beet's UTC calendar math.
///
/// The serializable counterpart of [`Instant`]: that clock is monotonic (elapsed
/// from an arbitrary process-local zero), so it is meaningless once written to a
/// store and read back in another process. This one is absolute, and its ordering
/// survives the round trip, which is what a persisted `created` field needs.
///
/// Signed because history did not start in 1970: a `Duration` cannot name
/// `1969-07-20`, so the epoch offset is an [`i64`] of milliseconds instead,
/// negative before the epoch. Milliseconds because that is the resolution the
/// wall clock actually has (`Date::now()` on wasm) and the one every consumer
/// reads at, and the range is still ±292 million years. As a single signed
/// integer the derived [`Ord`] is the chronological order, there is one spelling
/// of the epoch, and the serialized form is one JSON number, which a table
/// backend can sort on.
///
/// Reads the cross-platform [`time_ext::now`] rather than `SystemTime`, so it
/// works on wasm and no_std alike.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Timestamp(i64);

impl Timestamp {
	/// Midnight UTC on `1970-01-01`, the zero this type counts from.
	pub const UNIX_EPOCH: Self = Self(0);

	const MILLIS_PER_DAY: i64 = 86_400_000;

	/// The current wall-clock time.
	///
	/// # Panics
	///
	/// Panics if no clock is available (see [`try_now`](Self::try_now)).
	pub fn now() -> Self {
		Self::from_millis(time_ext::now().as_millis() as i64)
	}

	/// The current wall-clock time, or an error if no clock is installed yet.
	///
	/// Prefer this over [`now`](Self::now) when the clock may still be loading,
	/// ie a bare target whose SNTP client has yet to sync.
	pub fn try_now() -> Result<Self> {
		time_ext::try_now().map(|now| Self::from_millis(now.as_millis() as i64))
	}

	/// An instant `millis` after the Unix epoch, negative before it, for a time
	/// that came from somewhere other than the clock (a decoded uuid, a parsed
	/// header).
	pub fn from_millis(millis: i64) -> Self { Self(millis) }

	/// Milliseconds since the Unix epoch, negative before it.
	pub fn millis(&self) -> i64 { self.0 }

	/// An instant `secs` after the Unix epoch, negative before it.
	pub fn from_secs(secs: i64) -> Self { Self(secs * 1_000) }

	/// Whole seconds since the Unix epoch, rounded towards the epoch's past so
	/// that an instant always belongs to the second it falls in.
	pub fn secs(&self) -> i64 { self.0.div_euclid(1_000) }

	/// This instant's UTC civil `(year, month, day)`.
	///
	/// The days-to-civil algorithm (Howard Hinnant) directly rather than a
	/// datetime crate.
	pub fn civil_date(&self) -> (i64, u32, u32) {
		let days = self.0.div_euclid(Self::MILLIS_PER_DAY) + 719_468;
		let era = days.div_euclid(146_097);
		let doe = (days - era * 146_097) as u64;
		let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
		let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
		let mp = (5 * doy + 2) / 153;
		let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
		let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
		(yoe as i64 + era * 400 + (month <= 2) as i64, month, day)
	}

	/// Midnight UTC on a `YYYY-MM-DD` date, ie the instant a date-only field
	/// (a post's `created`, an archive key) names. A leading `-` is a negative
	/// year rather than a separator. `None` on any other shape.
	///
	/// The civil-to-days half of the algorithm [`civil_date`](Self::civil_date)
	/// inverts, so a stored date key (an aggregate row, an archive object) reads
	/// back as an instant without a datetime crate.
	pub fn parse_date(date: &str) -> Option<Self> {
		let (sign, rest) = match date.strip_prefix('-') {
			Some(rest) => (-1, rest),
			None => (1, date),
		};
		let mut parts = rest.split('-');
		let (year, month, day) = (
			sign * parts.next()?.parse::<i64>().ok()?,
			parts.next()?.parse::<u32>().ok()?,
			parts.next()?.parse::<u32>().ok()?,
		);
		if parts.next().is_some() || !(1..=12).contains(&month) || day == 0 {
			return None;
		}
		// civil-to-days (Howard Hinnant), the year shifted so march leads the year
		let year = year - (month <= 2) as i64;
		let era = year.div_euclid(400);
		let yoe = (year - era * 400) as u64;
		let mp = if month > 2 { month - 3 } else { month + 9 } as u64;
		let doy = (153 * mp + 2) / 5 + day as u64 - 1;
		let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
		(era * 146_097 + doe as i64 - 719_468)
			.checked_mul(Self::MILLIS_PER_DAY)
			.map(Self)
	}

	/// This instant's UTC date as `YYYY-MM-DD`, ie the key a daily aggregate row
	/// or archive object is named by, the inverse of
	/// [`parse_date`](Self::parse_date).
	pub fn format_date(&self) -> String {
		let (year, month, day) = self.civil_date();
		format!("{year:04}-{month:02}-{day:02}")
	}

	/// This instant's UTC date spelled for a reader rather than a key, eg
	/// `6 September 2025`.
	///
	/// Day-month-year with the month spelled out: unambiguous in every locale
	/// (where `06/09/2025` is not), and free of the ordinal suffix and comma that
	/// make `6th September, 2025` read like handwriting.
	pub fn format_long_date(&self) -> String {
		let (year, month, day) = self.civil_date();
		match Self::month_name(month) {
			Some(name) => format!("{day} {name} {year}"),
			// unreachable via `civil_date`, whose month is always 1..=12
			None => self.format_date(),
		}
	}

	/// This instant as an ISO 8601 / RFC 3339 UTC timestamp with millisecond
	/// precision, eg `2024-09-09T19:46:02.102Z`.
	pub fn format_iso8601(&self) -> String {
		let millis_of_day = self.0.rem_euclid(Self::MILLIS_PER_DAY);
		let secs = millis_of_day / 1_000;
		let (hour, min, sec) = (secs / 3_600, (secs / 60) % 60, secs % 60);
		let millis = millis_of_day % 1_000;
		format!(
			"{}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z",
			self.format_date()
		)
	}

	/// The English name of a month, `1..=12`. `None` outside that range.
	pub fn month_name(month: u32) -> Option<&'static str> {
		Self::MONTH_NAMES
			.get(month.checked_sub(1)? as usize)
			.copied()
	}

	/// The month names [`month_name`](Self::month_name) indexes, January first.
	const MONTH_NAMES: [&'static str; 12] = [
		"January",
		"February",
		"March",
		"April",
		"May",
		"June",
		"July",
		"August",
		"September",
		"October",
		"November",
		"December",
	];
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A date-only field round-trips through the instant it names, and anything
	/// that is not a date has none.
	#[crate::test]
	fn parses_and_formats_dates() {
		let created = Timestamp::parse_date("2025-09-06").unwrap();
		created.format_date().xpect_eq("2025-09-06");
		created.format_long_date().xpect_eq("6 September 2025");
		Timestamp::parse_date("someday").xpect_none();
	}

	/// The two halves of the civil algorithm agree across the epoch range, in
	/// both directions from the epoch.
	#[crate::test]
	fn parses_date() {
		for secs in (-2_000_000_000..2_000_000_000i64).step_by(86_400 * 37) {
			let timestamp = Timestamp::from_secs(secs);
			Timestamp::parse_date(&timestamp.format_date())
				.unwrap()
				.xpect_eq(Timestamp::from_secs(secs - secs.rem_euclid(86_400)));
		}
		Timestamp::parse_date("2024-02-29")
			.unwrap()
			.xpect_eq(Timestamp::from_secs(1_709_164_800));
		for date in ["2024-13-01", "2024-02", "not-a-date", ""] {
			Timestamp::parse_date(date).xpect_none();
		}
	}

	/// Dates before 1970 are instants like any other: negative, ordered before
	/// the epoch, and round-tripping through their key.
	#[crate::test]
	fn handles_pre_epoch() {
		let moon = Timestamp::parse_date("1969-07-20").unwrap();
		moon.millis().xpect_less_than(0);
		moon.xpect_less_than(Timestamp::UNIX_EPOCH);
		moon.format_date().xpect_eq("1969-07-20");
		moon.format_long_date().xpect_eq("20 July 1969");
		moon.format_iso8601().xpect_eq("1969-07-20T00:00:00.000Z");
		// the last millisecond of a pre-epoch day still belongs to that day
		Timestamp::from_millis(-1)
			.format_iso8601()
			.xpect_eq("1969-12-31T23:59:59.999Z");
		// a negative year is a sign, not a separator
		Timestamp::parse_date("-0044-03-15")
			.unwrap()
			.format_date()
			.xpect_eq("-044-03-15");
	}

	#[crate::test]
	fn formats_iso8601() {
		Timestamp::UNIX_EPOCH
			.format_iso8601()
			.xpect_eq("1970-01-01T00:00:00.000Z");
		Timestamp::from_millis(1_725_911_162_102)
			.format_iso8601()
			.xpect_eq("2024-09-09T19:46:02.102Z");
		// leap year day
		Timestamp::from_secs(1_709_164_800)
			.format_iso8601()
			.xpect_eq("2024-02-29T00:00:00.000Z");
	}

	#[crate::test]
	fn formats_date() {
		Timestamp::UNIX_EPOCH.format_date().xpect_eq("1970-01-01");
		Timestamp::UNIX_EPOCH
			.format_long_date()
			.xpect_eq("1 January 1970");
		// the last millisecond of a day still belongs to that day
		Timestamp::from_millis(1_725_926_399_999)
			.format_date()
			.xpect_eq("2024-09-09");
		Timestamp::from_millis(1_725_926_400_000)
			.format_date()
			.xpect_eq("2024-09-10");
		// leap year day
		Timestamp::from_secs(1_709_164_800)
			.format_long_date()
			.xpect_eq("29 February 2024");
	}
}
