use crate::prelude::*;

/// A wall-clock instant, stored as the signed milliseconds since the Unix epoch:
/// the moment a machine records, where a [`Date`] is the day a person writes.
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
/// Strict in what it is authored from: no markup or frontmatter string coerces
/// to one, since a person writes a day, which is a [`Date`]. The ISO 8601 and
/// RFC formats below are for machine interchange, and a field a reviewer reads
/// as text opts into ISO 8601 with a serde adapter of its own.
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

	/// Parse an ISO 8601 / RFC 3339 UTC timestamp, the inverse of
	/// [`format_iso8601`](Self::format_iso8601): a [`Date`], `T`, `HH:MM:SS`
	/// with optional fractional seconds, and a `Z`.
	/// `None` on any other shape, an offset included: a stored instant is UTC.
	pub fn parse_iso8601(text: &str) -> Option<Self> {
		let (date, time) = text.trim().split_once('T')?;
		let time = time.strip_suffix('Z')?;
		let (clock, fraction) = time
			.split_once('.')
			.map(|(clock, fraction)| (clock, Some(fraction)))
			.unwrap_or((time, None));
		let mut parts = clock.split(':');
		let (hour, min, sec) = (
			parts.next()?.parse::<i64>().ok()?,
			parts.next()?.parse::<i64>().ok()?,
			parts.next()?.parse::<i64>().ok()?,
		);
		if parts.next().is_some() || hour > 23 || min > 59 || sec > 60 {
			return None;
		}
		// a fraction of any length, read at millisecond precision
		let millis = match fraction {
			Some(fraction) if !fraction.is_empty() => {
				let digits = fraction
					.chars()
					.all(|char| char.is_ascii_digit())
					.then_some(fraction)?;
				format!("{digits:0<3}")[..3].parse::<i64>().ok()?
			}
			Some(_) => return None,
			None => 0,
		};
		Date::parse(date)
			.ok()?
			.timestamp()
			.0
			.checked_add(((hour * 60 + min) * 60 + sec) * 1_000 + millis)
			.map(Self)
	}

	/// [`parse_iso8601`](Self::parse_iso8601) also accepting a numeric
	/// offset (`2026-09-15T10:00:00.5+10:00`, the form an api reports a
	/// modified date in), read as the UTC instant it names.
	pub fn parse_rfc3339(text: &str) -> Option<Self> {
		let text = text.trim();
		if text.ends_with('Z') {
			return Self::parse_iso8601(text);
		}
		// the offset sign is the last `+` or `-` after the `T`
		let time_at = text.find('T')?;
		let sign_at = text.rfind(['+', '-']).filter(|at| *at > time_at)?;
		let (hours, minutes) = text[sign_at + 1..].split_once(':')?;
		let offset = hours.parse::<i64>().ok()? * 3600
			+ minutes.parse::<i64>().ok()? * 60;
		let offset = match &text[sign_at..=sign_at] {
			"+" => offset,
			_ => -offset,
		};
		Self::parse_iso8601(&format!("{}Z", &text[..sign_at]))?
			.0
			.checked_sub(offset * 1_000)
			.map(Self)
	}

	/// This instant as an ISO 8601 / RFC 3339 UTC timestamp with millisecond
	/// precision, eg `2024-09-09T19:46:02.102Z`.
	pub fn format_iso8601(&self) -> String {
		let (hour, min, sec, millis) = self.civil_time();
		format!(
			"{}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z",
			Date::from(*self)
		)
	}

	/// This instant as an ISO 8601 / RFC 3339 UTC timestamp at whole-second
	/// precision, eg `2024-09-09T19:46:02Z`: the form an api that names no
	/// fraction asks for. Truncated towards the second the instant falls in.
	pub fn format_iso8601_secs(&self) -> String {
		let (hour, min, sec, _) = self.civil_time();
		format!("{}T{hour:02}:{min:02}:{sec:02}Z", Date::from(*self))
	}

	/// This instant as an RFC 2822 date-time in UTC, eg
	/// `Mon, 08 Sep 2026 00:00:00 GMT` — the format an RSS `pubDate` requires.
	///
	/// Always `GMT` rather than `+0000`: both are legal, and the named zone is
	/// what every feed in the wild emits.
	pub fn format_rfc2822(&self) -> String {
		let date = Date::from(*self);
		let (year, month, day) = date.civil();
		let (hour, min, sec, _) = self.civil_time();
		let month = Date::month_name(month).map_or("Jan", |name| &name[..3]);
		format!(
			"{}, {day:02} {month} {year:04} {hour:02}:{min:02}:{sec:02} GMT",
			date.weekday_abbr()
		)
	}

	/// This instant's UTC `(hour, minute, second, millisecond)`.
	fn civil_time(&self) -> (i64, i64, i64, i64) {
		let millis_of_day = self.0.rem_euclid(Date::MILLIS_PER_DAY);
		let secs = millis_of_day / 1_000;
		(
			secs / 3_600,
			(secs / 60) % 60,
			secs % 60,
			millis_of_day % 1_000,
		)
	}
}

/// The instant `duration` later, saturating at the far future.
impl core::ops::Add<Duration> for Timestamp {
	type Output = Self;
	fn add(self, duration: Duration) -> Self {
		Self(self.0.saturating_add(
			duration.as_millis().try_into().unwrap_or(i64::MAX),
		))
	}
}

/// Serde for a [`Timestamp`] as ISO 8601 text rather than an epoch integer,
/// for a field a person reads (`modified = "2026-09-18T06:00:00.000Z"`) or a
/// wire that names its format (an atproto `datetime`). Use via
/// `#[serde(with = "beet_core::prelude::timestamp_iso8601")]`, or its
/// [`option`](timestamp_iso8601::option) module over an `Option`.
///
/// Writes [`Timestamp::format_iso8601`], UTC with milliseconds, which is also
/// the form an atproto `datetime` recommends. Reads any RFC 3339 instant
/// ([`Timestamp::parse_rfc3339`]), an offset included, since a foreign record
/// may carry one.
#[cfg(feature = "serde")]
pub mod timestamp_iso8601 {
	use crate::prelude::*;
	use serde::Deserializer;
	use serde::Serializer;
	use serde::de::Error;

	/// Serialize as ISO 8601 UTC text.
	pub fn serialize<S: Serializer>(
		value: &Timestamp,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&value.format_iso8601())
	}

	/// Deserialize RFC 3339 text, an offset read as the instant it names.
	pub fn deserialize<'de, D: Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Timestamp, D::Error> {
		let text = <alloc::borrow::Cow<str>>::deserialize(deserializer)?;
		Timestamp::parse_rfc3339(&text).ok_or_else(|| {
			D::Error::custom(format!(
				"`{text}` is not an RFC 3339 timestamp, ie \
				`2026-09-18T06:00:00.000Z`"
			))
		})
	}

	/// The same over an `Option`, `None` never written: pair it with
	/// `default` and `skip_serializing_if = "Option::is_none"`.
	pub mod option {
		use super::*;

		/// Serialize a present value as ISO 8601 UTC text.
		pub fn serialize<S: Serializer>(
			value: &Option<Timestamp>,
			serializer: S,
		) -> core::result::Result<S::Ok, S::Error> {
			match value {
				Some(value) => super::serialize(value, serializer),
				None => serializer.serialize_none(),
			}
		}

		/// Deserialize a present value from RFC 3339 text.
		pub fn deserialize<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> core::result::Result<Option<Timestamp>, D::Error> {
			super::deserialize(deserializer).map(Some)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Instants before 1970 are instants like any other: negative, ordered
	/// before the epoch, and formatting on the day they fall in.
	#[crate::test]
	fn handles_pre_epoch() {
		let moon = Date::parse("1969-07-20").unwrap().timestamp();
		moon.millis().xpect_less_than(0);
		moon.xpect_less_than(Timestamp::UNIX_EPOCH);
		moon.format_iso8601().xpect_eq("1969-07-20T00:00:00.000Z");
		// the last millisecond of a pre-epoch day still belongs to that day
		Timestamp::from_millis(-1)
			.format_iso8601()
			.xpect_eq("1969-12-31T23:59:59.999Z");
	}

	#[crate::test]
	fn formats_iso8601() {
		Timestamp::UNIX_EPOCH
			.format_iso8601()
			.xpect_eq("1970-01-01T00:00:00.000Z");
		Timestamp::from_millis(1_725_911_162_102)
			.format_iso8601()
			.xpect_eq("2024-09-09T19:46:02.102Z");
		// whole seconds, the fraction truncated rather than rounded up
		Timestamp::from_millis(1_725_911_162_902)
			.format_iso8601_secs()
			.xpect_eq("2024-09-09T19:46:02Z");
		// leap year day
		Timestamp::from_secs(1_709_164_800)
			.format_iso8601()
			.xpect_eq("2024-02-29T00:00:00.000Z");
	}

	/// The instant round-trips through its ISO 8601 text, and anything short
	/// of a full UTC timestamp is refused.
	#[crate::test]
	fn parses_iso8601() {
		for millis in [0, 1_725_911_162_102, -1, 1_709_164_800_000] {
			let timestamp = Timestamp::from_millis(millis);
			Timestamp::parse_iso8601(&timestamp.format_iso8601())
				.unwrap()
				.xpect_eq(timestamp);
		}
		Timestamp::parse_iso8601("2024-09-09T19:46:02Z")
			.unwrap()
			.xpect_eq(Timestamp::from_secs(1_725_911_162));
		Timestamp::parse_iso8601("2024-09-09T19:46:02.1Z")
			.unwrap()
			.xpect_eq(Timestamp::from_millis(1_725_911_162_100));
		for text in [
			"2024-09-09",
			"2024-09-09T19:46:02",
			"2024-09-09T19:46:02+10:00",
			"2024-09-09T25:00:00Z",
			"2024-09-09T19:46:02.Z",
		] {
			Timestamp::parse_iso8601(text).xpect_none();
		}
	}

	/// An offset lands on the UTC instant it names, whichever side of zero
	/// it sits, and `Z` still reads.
	#[crate::test]
	fn parses_rfc3339_offsets() {
		Timestamp::parse_rfc3339("2026-09-15T10:00:00.000000+10:00")
			.unwrap()
			.format_iso8601()
			.xpect_eq("2026-09-15T00:00:00.000Z");
		Timestamp::parse_rfc3339("2026-09-15T00:00:00-01:30")
			.unwrap()
			.format_iso8601()
			.xpect_eq("2026-09-15T01:30:00.000Z");
		Timestamp::parse_rfc3339("2024-09-09T19:46:02Z")
			.unwrap()
			.xpect_eq(Timestamp::from_secs(1_725_911_162));
		Timestamp::parse_rfc3339("2024-09-09T19:46:02").xpect_none();
		Timestamp::parse_rfc3339("yesterday").xpect_none();
	}

	/// The feed date format: an RFC 2822 date-time with the weekday its epoch
	/// day implies, on both sides of the epoch.
	#[crate::test]
	fn formats_rfc2822() {
		Timestamp::UNIX_EPOCH
			.format_rfc2822()
			.xpect_eq("Thu, 01 Jan 1970 00:00:00 GMT");
		Timestamp::from_millis(1_725_911_162_102)
			.format_rfc2822()
			.xpect_eq("Mon, 09 Sep 2024 19:46:02 GMT");
		Date::parse("1969-07-20")
			.unwrap()
			.timestamp()
			.format_rfc2822()
			.xpect_eq("Sun, 20 Jul 1969 00:00:00 GMT");
	}
}
