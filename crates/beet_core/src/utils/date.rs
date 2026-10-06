use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// A UTC calendar day, spelled `YYYY-MM-DD` in every text form, serde
/// included: the date-only twin of [`Timestamp`] for a field a person reads
/// and writes, ie a document's `updated` or the day a grade was given.
///
/// Held as the [`Timestamp`] of the day's midnight, so the derived [`Ord`] is
/// the chronological order and the calendar math stays with [`Timestamp`].
/// Reflect-opaque for the reason [`Url`] is: a date is a scalar to whoever
/// authors one, so a schema renders it as a text input and its serde form is
/// that same string, never the millisecond count a [`Timestamp`] writes.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let date = Date::parse("2026-10-02").unwrap();
/// date.to_string().xpect_eq("2026-10-02");
/// (date < Date::parse("2026-10-03").unwrap()).xpect_true();
/// ```
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect,
)]
#[reflect(opaque)]
#[reflect(Debug, Default, PartialEq, Hash)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Date(Timestamp);

impl Date {
	/// The current UTC day.
	///
	/// # Panics
	///
	/// Panics if no clock is available, as [`Timestamp::now`] does.
	pub fn today() -> Self { Timestamp::now().into() }

	/// Parse `YYYY-MM-DD`, the one spelling this type writes.
	///
	/// # Errors
	///
	/// Errors on any other shape, unpadded fields included, and on a day the
	/// month does not have, ie `2026-02-30`.
	pub fn parse(text: &str) -> Result<Self> {
		Timestamp::parse_date(text)
			.filter(|timestamp| timestamp.format_date() == text)
			.map(Self)
			.ok_or_else(|| {
				bevyhow!("`{text}` is not a date, expected YYYY-MM-DD")
			})
	}

	/// Midnight UTC, the instant this day begins.
	pub fn timestamp(&self) -> Timestamp { self.0 }
}

/// The day `timestamp` falls in.
impl From<Timestamp> for Date {
	fn from(timestamp: Timestamp) -> Self { Self(timestamp.start_of_day()) }
}

impl fmt::Display for Date {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(&self.0.format_date())
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
		serializer.serialize_str(&self.0.format_date())
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

	/// An instant anywhere in a day names that day, before the epoch too.
	#[crate::test]
	fn truncates_to_the_day() {
		let noon = Timestamp::parse_iso8601("2026-10-02T12:30:00Z").unwrap();
		Date::from(noon).xpect_eq(Date::parse("2026-10-02").unwrap());
		Date::from(Timestamp::from_secs(-1))
			.to_string()
			.xpect_eq("1969-12-31");
		for bad in ["2026-13-02", "2026-02-30", "2026-2-3", "someday"] {
			Date::parse(bad).xpect_err();
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
