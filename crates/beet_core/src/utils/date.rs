use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// A UTC calendar day, spelled `YYYY-MM-DD` in every text form, serde
/// included, and the owner of beet's calendar math: the day a person writes,
/// where a [`Timestamp`] is the instant a machine records.
///
/// Held as the days since `1970-01-01`, negative before it, so the derived
/// [`Ord`] is the chronological order. Reflect-opaque for the reason [`Url`]
/// is: a date is a scalar to whoever authors one, so a schema renders it as a
/// text input, a markup or frontmatter string coerces to it through its literal
/// parser, and its serde form is that same string.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let date = Date::parse("2026-10-02").unwrap();
/// date.to_string().xpect_eq("2026-10-02");
/// date.format_long().xpect_eq("2 October 2026");
/// (date < Date::parse("2026-10-03").unwrap()).xpect_true();
/// ```
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect,
)]
#[reflect(opaque)]
#[reflect(Debug, Default, PartialEq, Hash)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Date(i64);

impl Date {
	/// The milliseconds in a UTC day, which has no leap seconds.
	pub(crate) const MILLIS_PER_DAY: i64 = 86_400_000;

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

	/// The current UTC day.
	///
	/// # Panics
	///
	/// Panics if no clock is available, as [`Timestamp::now`] does.
	pub fn today() -> Self { Timestamp::now().into() }

	/// The day `year-month-day` names, a negative year before year 1.
	///
	/// The civil-to-days algorithm (Howard Hinnant) directly rather than a
	/// datetime crate, the inverse of [`civil`](Self::civil).
	///
	/// # Errors
	///
	/// Errors on a month outside `1..=12` or a day the month does not have, ie
	/// `2026-02-30`.
	pub fn from_civil(year: i64, month: u32, day: u32) -> Result<Self> {
		if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
			bevybail!("{year}-{month}-{day} is not a calendar day");
		}
		// the year shifted so march leads it, putting a leap day last
		let shifted = year - (month <= 2) as i64;
		let era = shifted.div_euclid(400);
		let yoe = (shifted - era * 400) as u64;
		let mp = if month > 2 { month - 3 } else { month + 9 } as u64;
		let doy = (153 * mp + 2) / 5 + day as u64 - 1;
		let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
		let date = Self(era * 146_097 + doe as i64 - 719_468);
		// a day past the month's end lands in the next month
		match date.civil() == (year, month, day) {
			true => date.xok(),
			false => bevybail!("{year}-{month}-{day} is not a calendar day"),
		}
	}

	/// Parse `YYYY-MM-DD`, the one spelling this type writes, a leading `-`
	/// being a negative year rather than a separator.
	///
	/// # Errors
	///
	/// Errors on any other shape, unpadded fields included, and on a day the
	/// month does not have.
	pub fn parse(text: &str) -> Result<Self> {
		Self::parse_fields(text)
			.filter(|date| date.to_string() == text)
			.ok_or_else(|| {
				bevyhow!("`{text}` is not a date, expected YYYY-MM-DD")
			})
	}

	/// The three dash-separated fields of `text` as a day, however padded.
	fn parse_fields(text: &str) -> Option<Self> {
		let (sign, rest) = match text.strip_prefix('-') {
			Some(rest) => (-1, rest),
			None => (1, text),
		};
		let mut fields = rest.split('-');
		let (year, month, day) = (
			sign * fields.next()?.parse::<i64>().ok()?,
			fields.next()?.parse::<u32>().ok()?,
			fields.next()?.parse::<u32>().ok()?,
		);
		match fields.next() {
			Some(_) => None,
			None => Self::from_civil(year, month, day).ok(),
		}
	}

	/// This day's `(year, month, day)`.
	///
	/// The days-to-civil algorithm (Howard Hinnant), the inverse of
	/// [`from_civil`](Self::from_civil).
	pub fn civil(&self) -> (i64, u32, u32) {
		let days = self.0 + 719_468;
		let era = days.div_euclid(146_097);
		let doe = (days - era * 146_097) as u64;
		let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
		let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
		let mp = (5 * doy + 2) / 153;
		let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
		let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
		(yoe as i64 + era * 400 + (month <= 2) as i64, month, day)
	}

	/// Midnight UTC, the instant this day begins.
	pub fn timestamp(&self) -> Timestamp {
		Timestamp::from_millis(self.0 * Self::MILLIS_PER_DAY)
	}

	/// This day spelled for a reader rather than a key, ie `6 September 2025`.
	///
	/// Day-month-year with the month spelled out: unambiguous in every locale
	/// (where `06/09/2025` is not), and free of the ordinal suffix and comma that
	/// make `6th September, 2025` read like handwriting.
	pub fn format_long(&self) -> String {
		let (year, month, day) = self.civil();
		// `civil`'s month is always 1..=12
		let month = Self::month_name(month).unwrap_or_default();
		format!("{day} {month} {year}")
	}

	/// The three-letter English weekday, ie `Mon`.
	pub fn weekday_abbr(&self) -> &'static str {
		// `1970-01-01` was a Thursday, so the epoch day indexes from there
		const NAMES: [&str; 7] =
			["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
		NAMES[self.0.rem_euclid(7) as usize]
	}

	/// The English name of a month, `1..=12`. `None` outside that range.
	pub fn month_name(month: u32) -> Option<&'static str> {
		Self::MONTH_NAMES
			.get(month.checked_sub(1)? as usize)
			.copied()
	}
}

/// The day `timestamp` falls in, a pre-epoch instant included.
impl From<Timestamp> for Date {
	fn from(timestamp: Timestamp) -> Self {
		Self(timestamp.millis().div_euclid(Self::MILLIS_PER_DAY))
	}
}

/// `YYYY-MM-DD`, a negative year as ISO 8601 writes it, ie `-0044-03-15`.
impl fmt::Display for Date {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let (year, month, day) = self.civil();
		let sign = if year < 0 { "-" } else { "" };
		write!(formatter, "{sign}{:04}-{month:02}-{day:02}", year.abs())
	}
}

impl FromStr for Date {
	type Err = BevyError;
	fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Date {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Date {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		let text = alloc::string::String::deserialize(deserializer)?;
		Self::parse(&text).map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A day round-trips through its spelling and its long form, and nothing
	/// else spells one.
	#[crate::test]
	fn parses_and_formats() {
		let date = Date::parse("2025-09-06").unwrap();
		date.to_string().xpect_eq("2025-09-06");
		date.format_long().xpect_eq("6 September 2025");
		Date::parse("2024-02-29")
			.unwrap()
			.format_long()
			.xpect_eq("29 February 2024");
		for bad in ["2026-13-02", "2026-02-30", "2026-2-3", "someday", ""] {
			Date::parse(bad).xpect_err();
		}
	}

	/// The two halves of the civil algorithm agree across the epoch range, in
	/// both directions from the epoch, and an instant anywhere in a day names
	/// that day.
	#[crate::test]
	fn civil_halves_agree() {
		for secs in (-2_000_000_000..2_000_000_000i64).step_by(86_400 * 37) {
			let date = Date::from(Timestamp::from_secs(secs));
			Date::parse(&date.to_string()).unwrap().xpect_eq(date);
			date.timestamp()
				.xpect_eq(Timestamp::from_secs(secs - secs.rem_euclid(86_400)));
		}
		// the last millisecond of a day still belongs to that day
		Date::from(Timestamp::from_millis(1_725_926_399_999))
			.to_string()
			.xpect_eq("2024-09-09");
		Date::from(Timestamp::from_millis(1_725_926_400_000))
			.to_string()
			.xpect_eq("2024-09-10");
	}

	/// Days before 1970 are days like any other: ordered before the epoch, and
	/// a negative year is a sign, not a separator.
	#[crate::test]
	fn handles_pre_epoch() {
		let moon = Date::parse("1969-07-20").unwrap();
		moon.timestamp().millis().xpect_less_than(0);
		moon.xpect_less_than(Date::default());
		moon.format_long().xpect_eq("20 July 1969");
		Date::from(Timestamp::from_secs(-1))
			.to_string()
			.xpect_eq("1969-12-31");
		Date::parse("-0044-03-15")
			.unwrap()
			.to_string()
			.xpect_eq("-0044-03-15");
	}

	/// The whole week, so no index is off by one.
	#[crate::test]
	fn names_weekdays() {
		for (offset, day) in ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"]
			.into_iter()
			.enumerate()
		{
			Date::from(Timestamp::from_secs(offset as i64 * 86_400))
				.weekday_abbr()
				.xpect_eq(day);
		}
	}

	/// The serde form is the `YYYY-MM-DD` string, and the schema reads it as
	/// one, so a stored date validates against the type that wrote it.
	#[cfg(feature = "serde")]
	#[crate::test]
	async fn serde_is_the_date_string() {
		let date = Date::parse("2026-10-02").unwrap();
		let mut value = Value::from_serde(date).unwrap();
		value.clone().xpect_eq(Value::Str("2026-10-02".into()));
		ValueSchema::of::<Date>()
			.assert_valid("date", &mut value)
			.await
			.unwrap();
		value.into_serde::<Date>().unwrap().xpect_eq(date);
	}
}
